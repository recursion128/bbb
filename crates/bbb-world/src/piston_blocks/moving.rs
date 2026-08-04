use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
    LevelEvent as ProtocolLevelEvent, Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::{
    block_name_is_air, BlockPos, ChunkPos, LevelEventSoundRandomState, PlayApplyEffects,
    SoundEventState, SoundHolderState, WorldStore,
};

use super::{PistonDirection, PistonStructureResolution};

const MOVING_PISTON_RETENTION_TICKS: u8 = 5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovingPistonState {
    pub pos: BlockPos,
    pub moved_block_state_id: i32,
    pub direction: PistonDirection,
    pub extending: bool,
    pub is_source: bool,
    pub progress_o: f32,
    pub progress: f32,
    pub death_ticks: u8,
}

impl MovingPistonState {
    pub fn progress_at(&self, partial_tick: f32) -> f32 {
        let partial_tick = partial_tick.min(1.0);
        self.progress_o + (self.progress - self.progress_o) * partial_tick
    }

    pub fn movement_direction(&self) -> PistonDirection {
        if self.extending {
            self.direction
        } else {
            self.direction.opposite()
        }
    }

    pub fn extended_progress(&self, progress: f32) -> f32 {
        if self.extending {
            progress - 1.0
        } else {
            1.0 - progress
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MovingPistonRenderBlockState {
    /// `PistonMovingBlockEntity.getBlockPos()`, i.e. what `shouldRender`
    /// measures against — not the movement-origin `block_pos` the bake uses.
    pub piston_pos: BlockPos,
    pub block_state_id: i32,
    pub random_seed_pos: BlockPos,
    pub block_pos: BlockPos,
    pub translation: [f32; 3],
}

#[derive(Debug)]
struct PistonMovePlan {
    updates: BTreeMap<BlockPos, i32>,
    records: Vec<MovingPistonState>,
    destroyed: Vec<(BlockPos, i32)>,
}

impl WorldStore {
    pub(crate) fn trigger_piston_block_event(
        &mut self,
        pos: BlockPos,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        event: ProtocolBlockEvent,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> bool {
        let Some(direction) = properties
            .get("facing")
            .and_then(|facing| PistonDirection::from_name(facing))
        else {
            return false;
        };
        let sticky = block_name == "minecraft:sticky_piston";
        match event.b0 {
            0 => self.extend_piston(pos, direction, sticky, random, effects),
            1 | 2 => {
                self.retract_piston(pos, direction, sticky, event.b0, event.b1, random, effects)
            }
            _ => false,
        }
    }

    fn extend_piston(
        &mut self,
        pos: BlockPos,
        direction: PistonDirection,
        sticky: bool,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> bool {
        let Some(mut plan) = self.plan_piston_move(pos, direction, true, sticky) else {
            return false;
        };
        let Some(base_state_id) = self.piston_base_state_id(pos, direction, true) else {
            return false;
        };
        plan.updates.insert(pos, base_state_id);
        if !self.apply_piston_plan(plan, random, effects) {
            return false;
        }
        self.emit_piston_sound(pos, true, random, effects);
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn retract_piston(
        &mut self,
        pos: BlockPos,
        direction: PistonDirection,
        sticky: bool,
        event_id: u8,
        event_param: u8,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> bool {
        let arm_pos = direction.relative(pos, 1);
        let two_pos = direction.relative(pos, 2);
        let source_block = if sticky {
            "minecraft:sticky_piston"
        } else {
            "minecraft:piston"
        };
        let source_direction = PistonDirection::from_legacy_id(event_param);
        let Some(source_state_id) = self.block_state_id(
            source_block,
            &[
                ("extended", "false"),
                ("facing", source_direction.as_name()),
            ],
        ) else {
            return false;
        };
        let Some(moving_source_state_id) = self.moving_piston_state_id(direction, sticky) else {
            return false;
        };

        let mut updates = BTreeMap::new();
        let mut records = Vec::new();
        let mut touched_records = BTreeSet::new();
        if let Some(record) = self.moving_piston(arm_pos).cloned() {
            let Some(final_state) = self.immediate_final_state_id(&record) else {
                return false;
            };
            updates.insert(arm_pos, final_state);
            touched_records.insert(arm_pos);
        }

        let piston_piece = sticky
            .then(|| self.moving_piston(two_pos))
            .flatten()
            .filter(|record| record.direction == direction && record.extending)
            .cloned();
        if let Some(record) = piston_piece.as_ref() {
            let Some(final_state) = self.immediate_final_state_id(record) else {
                return false;
            };
            updates.insert(two_pos, final_state);
            touched_records.insert(two_pos);
        }

        if piston_piece.is_none() {
            let pull = sticky
                && event_id == 1
                && self
                    .probe_block(two_pos)
                    .and_then(|block| block.block_name)
                    .is_some_and(|name| !block_name_is_air(&name));
            if pull {
                if let Some(plan) = self.plan_piston_move(pos, direction, false, sticky) {
                    updates.extend(plan.updates);
                    records.extend(plan.records);
                } else {
                    let Some(air) = self.air_block_state_id() else {
                        return false;
                    };
                    updates.insert(arm_pos, air);
                }
            } else {
                let Some(air) = self.air_block_state_id() else {
                    return false;
                };
                updates.insert(arm_pos, air);
            }
        }

        updates.insert(pos, moving_source_state_id);
        records.push(MovingPistonState {
            pos,
            moved_block_state_id: source_state_id,
            direction,
            extending: false,
            is_source: true,
            progress_o: 0.0,
            progress: 0.0,
            death_ticks: 0,
        });
        if !self.positions_are_loaded(updates.keys().copied()) {
            return false;
        }
        for touched in touched_records {
            self.moving_pistons.retain(|record| record.pos != touched);
        }
        for (update_pos, state_id) in updates {
            // Vanilla Level.setBlock no-ops on an identical state; only a
            // differing write may fail hard here.
            if self
                .probe_block(update_pos)
                .map(|block| block.block_state_id)
                == Some(state_id)
            {
                continue;
            }
            self.remove_block_entity_record_at(update_pos);
            if !self.set_block_state_id(update_pos, state_id) {
                return false;
            }
        }
        self.moving_pistons.extend(records);
        self.emit_piston_sound(pos, false, random, effects);
        true
    }

    fn plan_piston_move(
        &self,
        piston_pos: BlockPos,
        direction: PistonDirection,
        extending: bool,
        sticky: bool,
    ) -> Option<PistonMovePlan> {
        let resolution = self.resolve_piston_structure(piston_pos, direction, extending)?;
        self.plan_resolved_move(piston_pos, direction, extending, sticky, resolution)
    }

    fn plan_resolved_move(
        &self,
        piston_pos: BlockPos,
        direction: PistonDirection,
        extending: bool,
        sticky: bool,
        resolution: PistonStructureResolution,
    ) -> Option<PistonMovePlan> {
        let air = self.air_block_state_id()?;
        let actual_moving_state = self.moving_piston_state_id(direction, false)?;
        let mut updates = BTreeMap::new();
        let mut records = Vec::new();
        let mut destroyed = Vec::new();

        for destroy_pos in resolution.to_destroy.iter().rev().copied() {
            let state_id = self.probe_block(destroy_pos)?.block_state_id;
            destroyed.push((destroy_pos, state_id));
            updates.insert(destroy_pos, air);
        }

        let moved_states = resolution
            .to_push
            .iter()
            .map(|push_pos| Some((*push_pos, self.probe_block(*push_pos)?.block_state_id)))
            .collect::<Option<Vec<_>>>()?;
        for (source_pos, _) in &moved_states {
            updates.insert(*source_pos, air);
        }
        for (source_pos, moved_state_id) in moved_states.into_iter().rev() {
            let destination = resolution.push_direction.relative(source_pos, 1);
            updates.insert(destination, actual_moving_state);
            records.push(MovingPistonState {
                pos: destination,
                moved_block_state_id: moved_state_id,
                direction,
                extending,
                is_source: false,
                progress_o: 0.0,
                progress: 0.0,
                death_ticks: 0,
            });
        }

        if extending {
            let arm_pos = direction.relative(piston_pos, 1);
            let head_state = self.piston_head_state_id(direction, sticky, false)?;
            updates.insert(arm_pos, self.moving_piston_state_id(direction, sticky)?);
            records.push(MovingPistonState {
                pos: arm_pos,
                moved_block_state_id: head_state,
                direction,
                extending: true,
                is_source: true,
                progress_o: 0.0,
                progress: 0.0,
                death_ticks: 0,
            });
        }

        self.positions_are_loaded(updates.keys().copied())
            .then_some(PistonMovePlan {
                updates,
                records,
                destroyed,
            })
    }

    fn apply_piston_plan(
        &mut self,
        plan: PistonMovePlan,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) -> bool {
        for (pos, state_id) in &plan.updates {
            // Vanilla Level.setBlock no-ops on an identical state; only a
            // differing write may fail hard here.
            if self.probe_block(*pos).map(|block| block.block_state_id) == Some(*state_id) {
                continue;
            }
            self.remove_block_entity_record_at(*pos);
            if !self.set_block_state_id(*pos, *state_id) {
                return false;
            }
        }
        self.moving_pistons.extend(plan.records);
        for (pos, state_id) in plan.destroyed {
            self.emit_piston_destroy_level_event(
                ProtocolLevelEvent {
                    event_type: 2001,
                    pos: protocol_pos(pos),
                    data: state_id,
                    global: false,
                },
                random,
                effects,
            );
        }
        true
    }

    fn emit_piston_sound(
        &mut self,
        pos: BlockPos,
        extending: bool,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let (sound, pitch) = if extending {
            (
                "minecraft:block.piston.extend",
                random.next_float() * 0.25 + 0.6,
            )
        } else {
            (
                "minecraft:block.piston.contract",
                random.next_float() * 0.15 + 0.6,
            )
        };
        let state = self.record_positioned_sound(SoundEventState {
            sound: SoundHolderState {
                kind: "direct".to_string(),
                registry_id: None,
                location: Some(sound.to_string()),
                fixed_range: None,
            },
            source: "block".to_string(),
            position: ProtocolVec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume: 0.5,
            pitch,
            seed: random.next_long(),
            distance_delay: false,
        });
        effects.positioned_sound(&state);
    }

    pub fn moving_pistons(&self) -> &[MovingPistonState] {
        &self.moving_pistons
    }

    pub fn moving_piston(&self, pos: BlockPos) -> Option<&MovingPistonState> {
        self.moving_pistons.iter().find(|record| record.pos == pos)
    }

    pub fn moving_piston_render_blocks(
        &self,
        partial_tick: f32,
    ) -> Vec<MovingPistonRenderBlockState> {
        let mut blocks = Vec::new();
        for moving in &self.moving_pistons {
            let progress = moving.progress_at(partial_tick);
            let extended = moving.extended_progress(progress);
            let [dx, dy, dz] = moving.direction.step();
            let translation = [
                moving.pos.x as f32 + dx as f32 * extended,
                moving.pos.y as f32 + dy as f32 * extended,
                moving.pos.z as f32 + dz as f32 * extended,
            ];
            let render_pos = moving
                .movement_direction()
                .opposite()
                .relative(moving.pos, 1);
            let Some(moved_state) = self.registries.block_state(moving.moved_block_state_id) else {
                continue;
            };
            if block_name_is_air(&moved_state.name) {
                continue;
            }
            if moved_state.name == "minecraft:piston_head" {
                let sticky = moved_state
                    .properties
                    .get("type")
                    .is_some_and(|kind| kind == "sticky");
                let Some(state_id) =
                    self.piston_head_state_id(moving.direction, sticky, progress <= 0.5)
                else {
                    continue;
                };
                blocks.push(MovingPistonRenderBlockState {
                    piston_pos: moving.pos,
                    block_state_id: state_id,
                    random_seed_pos: render_pos,
                    block_pos: render_pos,
                    translation,
                });
            } else if moving.is_source && !moving.extending {
                let sticky = moved_state.name == "minecraft:sticky_piston";
                let source_direction = moved_state
                    .properties
                    .get("facing")
                    .and_then(|facing| PistonDirection::from_name(facing))
                    .unwrap_or(moving.direction);
                let Some(head_state_id) =
                    self.piston_head_state_id(source_direction, sticky, progress >= 0.5)
                else {
                    continue;
                };
                blocks.push(MovingPistonRenderBlockState {
                    piston_pos: moving.pos,
                    block_state_id: head_state_id,
                    random_seed_pos: render_pos,
                    block_pos: render_pos,
                    translation,
                });
                let mut base_properties = moved_state.properties.clone();
                base_properties.insert("extended".to_string(), "true".to_string());
                if let Some(base_state_id) = self
                    .registries
                    .block_state_id_by_name_and_properties(&moved_state.name, &base_properties)
                {
                    blocks.push(MovingPistonRenderBlockState {
                        piston_pos: moving.pos,
                        block_state_id: base_state_id,
                        random_seed_pos: moving.pos,
                        block_pos: moving.pos,
                        translation: [
                            moving.pos.x as f32,
                            moving.pos.y as f32,
                            moving.pos.z as f32,
                        ],
                    });
                }
            } else {
                blocks.push(MovingPistonRenderBlockState {
                    piston_pos: moving.pos,
                    block_state_id: moving.moved_block_state_id,
                    random_seed_pos: render_pos,
                    block_pos: render_pos,
                    translation,
                });
            }
        }
        blocks
    }

    pub fn advance_moving_piston_ticks(&mut self, ticks: u32) {
        for _ in 0..ticks {
            let running = self.moving_pistons.clone();
            let mut finalize = Vec::new();
            for snapshot in running {
                let Some(index) = self
                    .moving_pistons
                    .iter()
                    .position(|record| record.pos == snapshot.pos)
                else {
                    continue;
                };
                let progress_o = self.moving_pistons[index].progress;
                self.moving_pistons[index].progress_o = progress_o;
                if progress_o >= 1.0 {
                    if self.moving_pistons[index].death_ticks < MOVING_PISTON_RETENTION_TICKS {
                        self.moving_pistons[index].death_ticks += 1;
                    } else {
                        finalize.push(snapshot.pos);
                    }
                } else {
                    let new_progress = (progress_o + 0.5).min(1.0);
                    self.move_entities_for_piston_tick(&snapshot, new_progress);
                    if let Some(record) = self
                        .moving_pistons
                        .iter_mut()
                        .find(|record| record.pos == snapshot.pos)
                    {
                        record.progress = new_progress;
                    }
                }
            }
            for pos in finalize {
                self.complete_moving_piston(pos);
            }
        }
    }

    fn complete_moving_piston(&mut self, pos: BlockPos) {
        let Some(record) = self.moving_piston(pos).cloned() else {
            return;
        };
        self.moving_pistons.retain(|moving| moving.pos != pos);
        if self
            .probe_block(pos)
            .and_then(|block| block.block_name)
            .as_deref()
            != Some("minecraft:moving_piston")
        {
            return;
        }
        let final_state = self.final_tick_state_id(&record);
        if let Some(final_state) = final_state {
            let _ = self.set_block_state_id(pos, final_state);
        }
    }

    fn final_tick_state_id(&self, moving: &MovingPistonState) -> Option<i32> {
        let state = self.registries.block_state(moving.moved_block_state_id)?;
        if state.properties.get("waterlogged").map(String::as_str) != Some("true") {
            return Some(moving.moved_block_state_id);
        }
        let mut properties = state.properties.clone();
        properties.insert("waterlogged".to_string(), "false".to_string());
        self.registries
            .block_state_id_by_name_and_properties(&state.name, &properties)
    }

    fn immediate_final_state_id(&self, moving: &MovingPistonState) -> Option<i32> {
        if moving.is_source {
            self.air_block_state_id()
        } else {
            self.final_tick_state_id(moving)
        }
    }

    pub(crate) fn prune_moving_pistons_in_chunk(&mut self, chunk: ChunkPos) {
        self.moving_pistons.retain(|moving| {
            moving.pos.x.div_euclid(16) != chunk.x || moving.pos.z.div_euclid(16) != chunk.z
        });
    }

    fn positions_are_loaded(&self, positions: impl IntoIterator<Item = BlockPos>) -> bool {
        positions
            .into_iter()
            .all(|pos| self.probe_block(pos).is_some())
    }

    fn air_block_state_id(&self) -> Option<i32> {
        self.block_state_id("minecraft:air", &[])
    }

    fn moving_piston_state_id(&self, direction: PistonDirection, sticky: bool) -> Option<i32> {
        self.block_state_id(
            "minecraft:moving_piston",
            &[
                ("facing", direction.as_name()),
                ("type", if sticky { "sticky" } else { "normal" }),
            ],
        )
    }

    fn piston_head_state_id(
        &self,
        direction: PistonDirection,
        sticky: bool,
        short: bool,
    ) -> Option<i32> {
        self.block_state_id(
            "minecraft:piston_head",
            &[
                ("facing", direction.as_name()),
                ("short", if short { "true" } else { "false" }),
                ("type", if sticky { "sticky" } else { "normal" }),
            ],
        )
    }

    fn piston_base_state_id(
        &self,
        pos: BlockPos,
        direction: PistonDirection,
        extended: bool,
    ) -> Option<i32> {
        let block = self.probe_block(pos)?;
        let name = block.block_name?;
        self.block_state_id(
            &name,
            &[
                ("extended", if extended { "true" } else { "false" }),
                ("facing", direction.as_name()),
            ],
        )
    }

    fn block_state_id(&self, name: &str, properties: &[(&str, &str)]) -> Option<i32> {
        let properties = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>();
        self.registries
            .block_state_id_by_name_and_properties(name, &properties)
    }
}

fn protocol_pos(pos: BlockPos) -> ProtocolBlockPos {
    ProtocolBlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkColumn, ChunkSection, ChunkState, LightData, LocalPlayerPoseState, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::{
        block_types::{VANILLA_BLOCK_PISTON_ID, VANILLA_BLOCK_STICKY_PISTON_ID},
        packets::BlockUpdate,
    };

    #[derive(Default)]
    struct RecordingEffects {
        sounds: Vec<SoundEventState>,
        level_events: Vec<ProtocolLevelEvent>,
    }

    impl PlayApplyEffects for RecordingEffects {
        fn positioned_sound(&mut self, state: &SoundEventState) {
            self.sounds.push(state.clone());
        }

        fn level_event_particles(
            &mut self,
            _world: &WorldStore,
            event: &ProtocolLevelEvent,
            _random: &mut LevelEventSoundRandomState,
        ) -> bool {
            self.level_events.push(*event);
            false
        }
    }

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
                    crate::vanilla_air_block_state_id(),
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

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect::<BTreeMap<_, _>>();
        let state_id = world
            .registries
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("missing state {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: protocol_pos(pos),
            block_state_id: state_id,
        }));
    }

    fn block_event(pos: BlockPos, b0: u8, b1: u8, sticky: bool) -> ProtocolBlockEvent {
        ProtocolBlockEvent {
            pos: protocol_pos(pos),
            b0,
            b1,
            block_id: if sticky {
                VANILLA_BLOCK_STICKY_PISTON_ID
            } else {
                VANILLA_BLOCK_PISTON_ID
            },
        }
    }

    fn block_name(world: &WorldStore, pos: BlockPos) -> Option<String> {
        world.probe_block(pos).and_then(|block| block.block_name)
    }

    #[test]
    fn extension_mutates_atomically_emits_destroy_before_piston_sound_and_completes_after_retention(
    ) {
        let mut world = world_with_air_chunk();
        let base = pos(1, 5, 5);
        set_block(
            &mut world,
            base,
            "minecraft:piston",
            &[("extended", "false"), ("facing", "east")],
        );
        set_block(&mut world, pos(2, 5, 5), "minecraft:stone", &[]);
        set_block(&mut world, pos(3, 5, 5), "minecraft:torch", &[]);
        let mut random = LevelEventSoundRandomState::with_seed(4);
        let mut effects = RecordingEffects::default();

        assert!(world.apply_block_event_with_effects(
            block_event(base, 0, 0, false),
            &mut random,
            &mut effects,
        ));
        assert_eq!(
            world
                .probe_block(base)
                .unwrap()
                .block_properties
                .get("extended")
                .map(String::as_str),
            Some("true")
        );
        assert_eq!(
            block_name(&world, pos(2, 5, 5)).as_deref(),
            Some("minecraft:moving_piston")
        );
        assert_eq!(
            block_name(&world, pos(3, 5, 5)).as_deref(),
            Some("minecraft:moving_piston")
        );
        assert_eq!(world.moving_pistons().len(), 2);
        assert_eq!(effects.level_events.len(), 1);
        assert_eq!(effects.level_events[0].event_type, 2001);
        assert_eq!(effects.level_events[0].pos, protocol_pos(pos(3, 5, 5)));
        // The minimal world fixture has no asset-loaded block sound profiles,
        // so the internal 2001 still reaches the particle sink but contributes
        // no break sound. The piston sound itself is registry-independent.
        assert_eq!(effects.sounds.len(), 1);
        assert_eq!(
            effects.sounds[0].sound.location.as_deref(),
            Some("minecraft:block.piston.extend")
        );

        world.advance_moving_piston_ticks(2);
        assert!(world
            .moving_pistons()
            .iter()
            .all(|moving| moving.progress == 1.0));
        world.advance_moving_piston_ticks(5);
        assert_eq!(world.moving_pistons().len(), 2);
        world.advance_moving_piston_ticks(1);
        assert!(world.moving_pistons().is_empty());
        assert_eq!(
            block_name(&world, pos(2, 5, 5)).as_deref(),
            Some("minecraft:piston_head")
        );
        assert_eq!(
            block_name(&world, pos(3, 5, 5)).as_deref(),
            Some("minecraft:stone")
        );
    }

    #[test]
    fn sticky_retraction_moves_the_block_and_renders_source_head_plus_base() {
        let mut world = world_with_air_chunk();
        let base = pos(1, 5, 5);
        set_block(
            &mut world,
            base,
            "minecraft:sticky_piston",
            &[("extended", "true"), ("facing", "east")],
        );
        set_block(
            &mut world,
            pos(2, 5, 5),
            "minecraft:piston_head",
            &[("facing", "east"), ("short", "false"), ("type", "sticky")],
        );
        set_block(&mut world, pos(3, 5, 5), "minecraft:stone", &[]);

        // Synthetic mismatched b1 proves Vanilla's split ownership: movement
        // keeps the current state's east facing, while the retracting source
        // base/head model uses the packet's legacy north id.
        assert!(world.apply_block_event(block_event(base, 1, 2, true)));
        assert_eq!(
            block_name(&world, base).as_deref(),
            Some("minecraft:moving_piston")
        );
        assert_eq!(
            block_name(&world, pos(2, 5, 5)).as_deref(),
            Some("minecraft:moving_piston")
        );
        assert_eq!(
            block_name(&world, pos(3, 5, 5)).as_deref(),
            Some("minecraft:air")
        );
        assert_eq!(world.moving_pistons().len(), 2);
        let render_blocks = world.moving_piston_render_blocks(0.0);
        assert_eq!(render_blocks.len(), 3);
        assert!(render_blocks.iter().any(|block| {
            world
                .registries
                .block_state(block.block_state_id)
                .is_some_and(|state| {
                    state.name == "minecraft:piston_head"
                        && state.properties.get("facing").map(String::as_str) == Some("north")
                })
        }));

        world.advance_moving_piston_ticks(8);
        assert_eq!(
            block_name(&world, base).as_deref(),
            Some("minecraft:sticky_piston")
        );
        assert_eq!(
            world
                .probe_block(base)
                .unwrap()
                .block_properties
                .get("extended")
                .map(String::as_str),
            Some("false")
        );
        assert_eq!(
            world
                .probe_block(base)
                .unwrap()
                .block_properties
                .get("facing")
                .map(String::as_str),
            Some("north")
        );
        assert_eq!(
            block_name(&world, pos(2, 5, 5)).as_deref(),
            Some("minecraft:stone")
        );
    }

    #[test]
    fn ordinary_retraction_and_drop_remove_the_head_without_pulling() {
        for event_id in [1, 2] {
            let mut world = world_with_air_chunk();
            let base = pos(1, 5, 5);
            set_block(
                &mut world,
                base,
                "minecraft:piston",
                &[("extended", "true"), ("facing", "east")],
            );
            set_block(
                &mut world,
                pos(2, 5, 5),
                "minecraft:piston_head",
                &[("facing", "east"), ("short", "false"), ("type", "normal")],
            );
            set_block(&mut world, pos(3, 5, 5), "minecraft:stone", &[]);

            assert!(world.apply_block_event(block_event(base, event_id, 5, false)));
            assert_eq!(world.moving_pistons().len(), 1);
            assert_eq!(
                block_name(&world, pos(2, 5, 5)).as_deref(),
                Some("minecraft:air")
            );
            assert_eq!(
                block_name(&world, pos(3, 5, 5)).as_deref(),
                Some("minecraft:stone")
            );
        }
    }

    #[test]
    fn extension_fails_without_partial_mutation_when_destination_chunk_is_unloaded() {
        let mut world = world_with_air_chunk();
        let base = pos(14, 5, 5);
        set_block(
            &mut world,
            base,
            "minecraft:piston",
            &[("extended", "false"), ("facing", "east")],
        );
        set_block(&mut world, pos(15, 5, 5), "minecraft:stone", &[]);

        assert!(!world.apply_block_event(block_event(base, 0, 0, false)));
        assert_eq!(
            block_name(&world, base).as_deref(),
            Some("minecraft:piston")
        );
        assert_eq!(
            block_name(&world, pos(15, 5, 5)).as_deref(),
            Some("minecraft:stone")
        );
        assert!(world.moving_pistons().is_empty());
    }

    #[test]
    fn canonical_record_drives_dynamic_collision_and_entity_push() {
        let mut world = world_with_air_chunk();
        let base = pos(1, 5, 5);
        set_block(
            &mut world,
            base,
            "minecraft:piston",
            &[("extended", "false"), ("facing", "east")],
        );
        set_block(&mut world, pos(2, 5, 5), "minecraft:stone", &[]);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 3.1,
                y: 5.0,
                z: 5.5,
            },
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        assert!(world.apply_block_event(block_event(base, 0, 0, false)));
        let moving_pos = pos(3, 5, 5);
        assert!(world.local_player_pose_collides_with_block(
            moving_pos,
            LocalPlayerPoseState {
                position: ProtocolVec3d {
                    x: 2.7,
                    y: 5.0,
                    z: 5.5,
                },
                ..LocalPlayerPoseState::default()
            }
        ));
        world.advance_moving_piston_ticks(1);
        assert!((world.local_player_pose().unwrap().position.x - 3.61).abs() < 1.0e-9);
    }

    #[test]
    fn horizontal_honey_motion_carries_a_grounded_entity_on_top() {
        let mut world = world_with_air_chunk();
        let base = pos(1, 5, 5);
        set_block(
            &mut world,
            base,
            "minecraft:piston",
            &[("extended", "false"), ("facing", "east")],
        );
        set_block(&mut world, pos(2, 5, 5), "minecraft:honey_block", &[]);
        world.set_local_player_pose(LocalPlayerPoseState {
            position: ProtocolVec3d {
                x: 2.5,
                y: 5.9375,
                z: 5.5,
            },
            on_ground: true,
            ..LocalPlayerPoseState::default()
        });

        assert!(world.apply_block_event(block_event(base, 0, 0, false)));
        world.advance_moving_piston_ticks(1);
        assert!((world.local_player_pose().unwrap().position.x - 3.0).abs() < 1.0e-9);
    }
}
