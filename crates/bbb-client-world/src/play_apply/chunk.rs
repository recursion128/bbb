use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID;
use bbb_protocol::packets::{
    Explosion as ProtocolExplosion, GameEvent as ProtocolGameEvent,
    LevelEvent as ProtocolLevelEvent, Vec3d as ProtocolVec3d,
};

use crate::{LevelEventSoundRandomState, SoundEventState, WorldStore};

use super::PlayApplyEffects;

const COBWEB_PLACE_LEVEL_EVENT: i32 = 3018;
const DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT: i32 = 2006;
const INSTANT_POTION_BREAK_LEVEL_EVENT: i32 = 2007;
const PLANT_GROWTH_LEVEL_EVENT: i32 = 1505;
const POTION_BREAK_LEVEL_EVENT: i32 = 2002;
const VAULT_ACTIVATE_LEVEL_EVENT: i32 = 3015;
const VAULT_DEACTIVATE_LEVEL_EVENT: i32 = 3016;
const SCULK_SHRIEKER_LEVEL_EVENT: i32 = 3007;
const WAX_ON_LEVEL_EVENT: i32 = 3003;

impl WorldStore {
    pub(super) fn apply_play_explosion(
        &mut self,
        update: ProtocolExplosion,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let state = self.record_explosion(update);
        let sound = self.record_explosion_sound(&state, random);
        effects.positioned_sound(&sound);
        effects.explosion_particles(self, &state);
        self.apply_explosion_knockback(&state);
    }

    pub(super) fn apply_play_game_event(
        &mut self,
        update: ProtocolGameEvent,
        effects: &mut dyn PlayApplyEffects,
    ) {
        self.apply_game_event(update);
        if let Some(position) = self.elder_guardian_effect_particle_position(update) {
            effects.elder_guardian_effect_particles(self, position);
        }
        if let Some(state) = self.game_event_positioned_sound(update) {
            effects.positioned_sound(&state);
        }
    }

    fn elder_guardian_effect_particle_position(
        &self,
        event: ProtocolGameEvent,
    ) -> Option<ProtocolVec3d> {
        (event.event_id == VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID)
            .then(|| self.local_player_pose().map(|pose| pose.position))
            .flatten()
    }

    /// Vanilla `LevelRenderer.levelEvent` client effects: canonical state,
    /// jukebox/global/local/positioned sounds, and runtime particles.
    pub(super) fn apply_level_event_with_effects(
        &mut self,
        event: ProtocolLevelEvent,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        let jukebox_event = self.apply_level_event(event);
        if let Some(jukebox_event) = jukebox_event {
            effects.jukebox_level_event(&jukebox_event);
        }
        if let Some(state) = camera_audio_position(self)
            .and_then(|camera_position| self.global_level_event_sound(event, camera_position))
        {
            let state = self.record_positioned_sound(with_level_event_sound_seed(state, random));
            effects.positioned_sound(&state);
        }
        if let Some(state) = self
            .level_event_local_sound_with_random(event, || random.next_float())
            .map(|state| self.record_local_sound(state))
        {
            effects.local_sound(&state);
        }
        let particles_first = matches!(
            event.event_type,
            POTION_BREAK_LEVEL_EVENT
                | INSTANT_POTION_BREAK_LEVEL_EVENT
                | DRAGON_FIREBALL_EXPLODE_LEVEL_EVENT
                | WAX_ON_LEVEL_EVENT
                | PLANT_GROWTH_LEVEL_EVENT
                | COBWEB_PLACE_LEVEL_EVENT
                | SCULK_SHRIEKER_LEVEL_EVENT
                | VAULT_ACTIVATE_LEVEL_EVENT
                | VAULT_DEACTIVATE_LEVEL_EVENT
        );
        if particles_first {
            effects.level_event_particles(self, &event, random);
        }
        let sound = match event.event_type {
            COBWEB_PLACE_LEVEL_EVENT => {
                self.cobweb_place_level_event_sound_with_random(event, || random.next_float())
            }
            SCULK_SHRIEKER_LEVEL_EVENT => {
                self.sculk_shrieker_level_event_sound_with_random(event, || random.next_float())
            }
            VAULT_ACTIVATE_LEVEL_EVENT | VAULT_DEACTIVATE_LEVEL_EVENT => {
                self.vault_level_event_sound_with_random(event, || random.next_float())
            }
            _ => self.level_event_sound_with_random(event, || random.next_float()),
        };
        if let Some(state) = sound {
            let state = self.record_positioned_sound(with_level_event_sound_seed(state, random));
            effects.positioned_sound(&state);
        }
        if !particles_first {
            effects.level_event_particles(self, &event, random);
        }
    }

    /// Runs the client-local level-event 2001 emitted by piston destruction
    /// without counting it as a received `ClientboundLevelEventPacket`.
    pub(crate) fn emit_piston_destroy_level_event(
        &mut self,
        event: ProtocolLevelEvent,
        random: &mut LevelEventSoundRandomState,
        effects: &mut dyn PlayApplyEffects,
    ) {
        debug_assert_eq!(event.event_type, 2001);
        if let Some(state) = self.level_event_sound_with_random(event, || random.next_float()) {
            let state = self.record_positioned_sound(with_level_event_sound_seed(state, random));
            effects.positioned_sound(&state);
        }
        effects.level_event_particles(self, &event, random);
    }
}

/// Vanilla camera-relative audio position: the current camera entity when it
/// is not following the player, otherwise the local player's eye position.
fn camera_audio_position(world: &WorldStore) -> Option<ProtocolVec3d> {
    let camera = world.local_player().camera;
    if let Some(camera_id) = camera.entity_id {
        if !camera.follows_player {
            if let Some(camera_pose) = world.probe_entity_camera_pose(camera_id) {
                return Some(ProtocolVec3d {
                    x: camera_pose.position.x,
                    y: camera_pose.position.y + f64::from(camera_pose.eye_height),
                    z: camera_pose.position.z,
                });
            }
        }
    }

    world.local_player_pose().map(|pose| ProtocolVec3d {
        x: pose.position.x,
        y: pose.position.y + world.local_player_eye_height(pose),
        z: pose.position.z,
    })
}

fn with_level_event_sound_seed(
    mut state: SoundEventState,
    random: &mut LevelEventSoundRandomState,
) -> SoundEventState {
    state.seed = random.next_long();
    state
}
