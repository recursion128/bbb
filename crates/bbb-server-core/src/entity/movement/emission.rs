//! Shared Entity movement distance and ordered sound/vibration effects.

use crate::level_event::VanillaGameEvent;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::Vec3Ops;

use super::MovementEmission;

mod native;
mod sounds;
pub(super) use native::apply;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyMovementEmissionState {
    pub move_dist: f32,
    pub fly_dist: f32,
    pub next_step: f32,
}

impl Default for BodyMovementEmissionState {
    fn default() -> Self {
        Self {
            move_dist: 0.0,
            fly_dist: 0.0,
            next_step: 1.0,
        }
    }
}

/// What the moving body is standing on and doing when its emission runs.
#[derive(Debug, Clone, Copy)]
pub struct BodyMovementFacts {
    pub support: BlockPos,
    pub support_state: i32,
    pub on_ground: bool,
    pub crouching: bool,
    pub on_rails: bool,
    pub swimming: bool,
    pub in_water: bool,
}

/// One ordered side effect of a movement emission, applied by the body owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyMovementEffect {
    StepSound { pos: BlockPos, state: i32 },
    SwimSound,
    GameEvent(VanillaGameEvent, Option<i32>),
}

/// Entity.java:804-839: advances the distance counters and lists the effects
/// in emission order; `None` is an unresolved climbable tag.
pub fn body_movement_effects(
    state: &mut BodyMovementEmissionState,
    facts: BodyMovementFacts,
    climbable: impl Fn(i32) -> Option<bool>,
    emission: MovementEmission,
    clipped: Vec3d,
    effect_pos: BlockPos,
    effect_state: i32,
) -> Option<Vec<BodyMovementEffect>> {
    let mut effects = Vec::new();
    let moved = (clipped.length_sqr().sqrt() * f64::from(0.6_f32)) as f32;
    let horizontal = (clipped.horizontal_distance_sqr().sqrt() * f64::from(0.6_f32)) as f32;
    let climbing = climbable(facts.support_state)?;
    state.move_dist += if climbing { moved } else { horizontal };
    state.fly_dist += moved;
    if state.move_dist > state.next_step && !block_state_is_air(facts.support_state) {
        let same = facts.support == effect_pos;
        let mut produced = step_effects(
            &mut effects,
            &facts,
            &climbable,
            effect_pos,
            effect_state,
            emission.emits_sounds(),
            same,
            clipped,
        )?;
        if !same {
            produced |= step_effects(
                &mut effects,
                &facts,
                &climbable,
                facts.support,
                facts.support_state,
                false,
                emission.emits_events(),
                clipped,
            )?;
        }
        if produced || facts.in_water {
            state.next_step = (state.move_dist as i32) as f32 + 1.0;
            if !produced {
                if emission.emits_sounds() {
                    effects.push(BodyMovementEffect::SwimSound);
                }
                if emission.emits_events() {
                    effects.push(BodyMovementEffect::GameEvent(VanillaGameEvent::SWIM, None));
                }
            }
        }
    }
    Some(effects)
}

/// Entity.java:923-944.
#[allow(clippy::too_many_arguments)]
fn step_effects(
    effects: &mut Vec<BodyMovementEffect>,
    facts: &BodyMovementFacts,
    climbable: &impl Fn(i32) -> Option<bool>,
    pos: BlockPos,
    state: i32,
    sound: bool,
    vibrate: bool,
    clipped: Vec3d,
) -> Option<bool> {
    if block_state_is_air(state) {
        return Some(false);
    }
    if (facts.on_ground
        || climbable(state)?
        || facts.crouching && clipped.y == 0.0
        || facts.on_rails)
        && !facts.swimming
    {
        if sound {
            effects.push(BodyMovementEffect::StepSound { pos, state });
        }
        if vibrate {
            effects.push(BodyMovementEffect::GameEvent(
                VanillaGameEvent::STEP,
                Some(state),
            ));
        }
        return Some(true);
    }
    Some(false)
}

/// Entity.java:1346-1356; pitch RNG belongs to the emitter, even with a controller.
pub fn body_swim_sound_volume(movement: Vec3d, controlled: bool) -> f32 {
    ((movement.x * movement.x * f64::from(0.2_f32)
        + movement.y * movement.y
        + movement.z * movement.z * f64::from(0.2_f32))
    .sqrt() as f32
        * if controlled { 0.4_f32 } else { 0.35_f32 })
    .min(1.0)
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BodyCrystalSoundState {
    intensity: f32,
    last_tick: i32,
}

impl BodyCrystalSoundState {
    pub fn ready(&self, tick: i32) -> bool {
        tick >= self.last_tick.wrapping_add(20)
    }

    /// Entity.java:1385-1390. Silent suppresses publication, not this RNG/state.
    pub fn sound(&mut self, tick: i32, random: f32) -> (f32, f32) {
        self.intensity *= 0.997_f64.powf(f64::from(tick.wrapping_sub(self.last_tick))) as f32;
        self.intensity = (self.intensity + 0.07_f32).min(1.0);
        let pitch = 0.5_f32 + self.intensity * random * 1.2_f32;
        let volume = 0.1_f32 + self.intensity * 1.2_f32;
        self.last_tick = tick;
        (volume, pitch)
    }
}
