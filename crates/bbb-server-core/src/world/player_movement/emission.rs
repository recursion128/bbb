//! Actual session adaptation of Entity movement emission and Player step sounds.

use crate::connection::ConnectionId;
use crate::entity::{
    body_movement_effects, BodyMovementEffect, BodyMovementFacts, FluidTag, MovementEmission,
};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::block_sound_facts_26_1::vanilla_block_state_sound_type_26_1;
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{vanilla_block_type, VANILLA_BLOCK_POWDER_SNOW_ID};
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
};
use bbb_protocol::packets::{BlockPos, SoundEventHolder, SoundSource};

use super::{GameType, PlayerMoveProbe, ServerWorld, WorldEvent};

impl ServerWorld {
    pub(in crate::world) fn finish_player_move_emission(
        &mut self,
        id: ConnectionId,
        probe: &PlayerMoveProbe,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let player = self.levels.player(id)?;
        // Entity.java:706-710,790-795; Player.java:1619-1621.
        if player.game_type == GameType::Spectator
            || player.cold.last_vehicle.is_some()
            || player.abilities.flying
            || player.on_ground() && player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN)
        {
            return Some(());
        }
        let pos = self.player_legacy_support_pos_at(
            id,
            player.position(),
            player.block_contact.supporting,
        )?;
        let support = self.player_support_pos_at(
            id,
            player.position(),
            player.block_contact.supporting,
            1.0e-5_f32,
        )?;
        let mut body = Session {
            world: self,
            id,
            events,
        };
        let state = body.block_state(pos)?;
        let player = body.world.levels.player(id).unwrap();
        let facts = BodyMovementFacts {
            support,
            support_state: body.block_state(support)?,
            on_ground: player.on_ground(),
            crouching: player.pose_id
                == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID,
            on_rails: false,
            swimming: player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING),
            in_water: body.is_in_water(),
        };
        let mut counters = player.block_contact.movement_emission;
        let effects = body_movement_effects(
            &mut counters,
            facts,
            |state| body.is_climbable(state),
            MovementEmission::All,
            probe.solved_delta,
            pos,
            state,
        );
        body.world.levels[id].block_contact.movement_emission = counters;
        for effect in effects? {
            match effect {
                BodyMovementEffect::StepSound { pos, state } => {
                    body.walking_step_sound(pos, state)?;
                }
                BodyMovementEffect::SwimSound => body.water_swim_sound()?,
                BodyMovementEffect::GameEvent(event, state) => body.game_event(event, state),
            }
        }
        Some(())
    }
}

struct Session<'a> {
    world: &'a mut ServerWorld,
    id: ConnectionId,
    events: &'a mut Vec<WorldEvent>,
}

impl Session<'_> {
    fn block_state(&self, pos: BlockPos) -> Option<i32> {
        let player = self.world.levels.player(self.id).unwrap();
        let level = self.world.level_by_id(&player.dimension);
        if level.chunks.is_inside_build_height(pos.y) {
            level.chunks.get_block_state(pos.x, pos.y, pos.z)
        } else {
            Some(level.block_state(pos))
        }
    }
    fn tag(&self, state: i32, tag: &str) -> Option<bool> {
        let tags = self.world.player_registry_tags(self.id)?;
        if let Some(block) = vanilla_block_state_block_type_id(state).and_then(vanilla_block_type) {
            return Some(tags.contains("minecraft:block", tag, block.resource_id));
        }
        let (custom, _) = self
            .world
            .registries
            .content_registry()
            .block_by_state_id(state)?;
        Some(tags.contains("minecraft:block", tag, custom.resource_id()))
    }
    fn sound(&mut self, resource: &str, volume: f32, pitch: f32) -> Option<()> {
        let registry_id =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource)?;
        let position = self.world.levels[self.id].position();
        self.world.play_player_sound(
            self.id,
            Some(self.id),
            SoundEventHolder::Reference { registry_id },
            SoundSource::Players,
            position,
            volume,
            pitch,
            self.events,
        );
        Some(())
    }
    fn block_sound(&mut self, state: i32, muffled: bool) -> Option<()> {
        let sound = vanilla_block_state_sound_type_26_1(state);
        self.sound(
            sound.step_sound,
            sound.volume * if muffled { 0.05_f32 } else { 0.15_f32 },
            sound.pitch * if muffled { 0.8_f32 } else { 1.0_f32 },
        )
    }
    fn is_climbable(&self, state: i32) -> Option<bool> {
        Some(
            self.tag(state, "minecraft:climbable")?
                || vanilla_block_state_block_type_id(state) == Some(VANILLA_BLOCK_POWDER_SNOW_ID),
        )
    }
    fn is_in_water(&self) -> bool {
        self.world.levels[self.id]
            .fluid_interaction
            .is_in_fluid(FluidTag::Water)
    }
    fn walking_step_sound(&mut self, pos: BlockPos, state: i32) -> Option<()> {
        // Player.java:1461-1478; Entity.java:1339-1344,1359-1373.
        if self.is_in_water() {
            self.water_swim_sound()?;
            self.block_sound(state, true)?;
        } else {
            let above = BlockPos {
                y: pos.y + 1,
                ..pos
            };
            let above_state = self.block_state(above)?;
            if self.tag(above_state, "minecraft:inside_step_sound_blocks")?
                || self.tag(above_state, "minecraft:combination_step_sound_blocks")?
            {
                let combined = self.tag(above_state, "minecraft:combination_step_sound_blocks")?;
                self.block_sound(above_state, false)?;
                if combined {
                    self.block_sound(state, true)?;
                }
            } else {
                self.block_sound(state, false)?;
            }
        }
        let tick = self.world.levels[self.id].tick_count;
        if self.tag(state, "minecraft:crystal_sound_blocks")?
            && self.world.levels[self.id]
                .block_contact
                .crystal_sound
                .ready(tick)
        {
            let player = self.world.levels.player_mut(self.id)?;
            let random = self.world.random.next_float();
            let (volume, pitch) = player.block_contact.crystal_sound.sound(tick, random);
            self.sound("minecraft:block.amethyst_block.chime", volume, pitch)?;
        }
        Some(())
    }
    fn water_swim_sound(&mut self) -> Option<()> {
        let player = self.world.levels.player_mut(self.id)?;
        let volume = crate::entity::body_swim_sound_volume(player.delta_movement(), false);
        let pitch =
            1.0_f32 + (self.world.random.next_float() - self.world.random.next_float()) * 0.4_f32;
        self.sound("minecraft:entity.player.swim", volume, pitch)
    }
    fn game_event(&mut self, event: VanillaGameEvent, state: Option<i32>) {
        let player = self.world.levels.player(self.id).unwrap();
        let (dimension, position, entity_id) =
            (player.dimension, player.position(), player.entity_id());
        self.world
            .level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source_at(event, position, Some(entity_id), state);
    }
}
