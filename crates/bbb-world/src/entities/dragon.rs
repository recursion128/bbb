use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID;

use super::schema::{
    vanilla_ender_dragon_phase_is_sitting,
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type,
    VANILLA_ENDER_DRAGON_PHASES_26_1,
};
use super::{EntityPickBoundsState, EntityPickTargetState, EntityTransform, EntityVec3};

const DRAGON_FLIGHT_HISTORY_LEN: usize = 64;
const DRAGON_FLIGHT_HISTORY_MASK: i32 = 63;

const ENDER_DRAGON_PARTS: &[EnderDragonPartSpec] = &[
    EnderDragonPartSpec {
        id_offset: 1,
        width: 1.0,
        height: 1.0,
        offset: EnderDragonPartOffset::Head,
    },
    EnderDragonPartSpec {
        id_offset: 2,
        width: 3.0,
        height: 3.0,
        offset: EnderDragonPartOffset::Neck,
    },
    EnderDragonPartSpec {
        id_offset: 3,
        width: 5.0,
        height: 3.0,
        offset: EnderDragonPartOffset::Body,
    },
    EnderDragonPartSpec {
        id_offset: 4,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(0),
    },
    EnderDragonPartSpec {
        id_offset: 5,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(1),
    },
    EnderDragonPartSpec {
        id_offset: 6,
        width: 2.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Tail(2),
    },
    EnderDragonPartSpec {
        id_offset: 7,
        width: 4.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Wing(1.0),
    },
    EnderDragonPartSpec {
        id_offset: 8,
        width: 4.0,
        height: 2.0,
        offset: EnderDragonPartOffset::Wing(-1.0),
    },
];

#[derive(Debug, Clone, Copy)]
struct EnderDragonPartSpec {
    id_offset: i32,
    width: f32,
    height: f32,
    offset: EnderDragonPartOffset,
}

#[derive(Debug, Clone, Copy)]
enum EnderDragonPartOffset {
    Head,
    Neck,
    Body,
    Tail(u8),
    Wing(f64),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnderDragonAnimationState {
    #[serde(default = "default_ender_dragon_phase_id")]
    pub phase_id: i32,
    #[serde(default)]
    pub dragon_dying: bool,
    #[serde(default)]
    pub dragon_death_time: i32,
    #[serde(default)]
    pub y_rot_a: f32,
    /// Vanilla `EnderDragon.oFlapTime` / `flapTime`: the wing-beat phase `EnderDragonModel.setupAnim`
    /// scales by `2π`, advanced every client tick from the delta movement (or `+0.1` while sitting).
    #[serde(default)]
    pub o_flap_time: f32,
    #[serde(default)]
    pub flap_time: f32,
    /// Vanilla `Mob.isNoAi()` from the shared mob-flags metadata: a no-AI dragon pins
    /// `flapTime = 0.5` and stops recording flight history.
    #[serde(default)]
    pub no_ai: bool,
    #[serde(default)]
    pub flight_history: DragonFlightHistoryState,
    /// Vanilla `EnderDragon.growlTime`, initialized to 100 and decremented only
    /// by the audible, non-sitting client branch. It is client-private runtime
    /// state, so stable probes omit it while retained generations still clone it.
    #[serde(skip)]
    pub(crate) growl_time: Option<i32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct EnderDragonClientSoundTriggers {
    /// `Entity.processFlappingMovement` / `EnderDragon.onFlap`, before `aiStep`
    /// advances either flap endpoint.
    pub(crate) flap_pitch: Option<f32>,
    /// Either the non-sitting ambient growl or
    /// `DragonSittingAttackingPhase.doClientTick`; those branches are mutually
    /// exclusive in one tick.
    pub(crate) growl_pitch: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragonFlightHistoryState {
    pub samples: [DragonFlightHistorySample; DRAGON_FLIGHT_HISTORY_LEN],
    pub head: i8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DragonFlightHistorySample {
    pub y: f64,
    pub y_rot: f32,
}

#[derive(Deserialize)]
struct DragonFlightHistoryStateWire {
    #[serde(default)]
    samples: Vec<DragonFlightHistorySample>,
    #[serde(default = "default_flight_history_head")]
    head: i8,
}

impl Default for EnderDragonAnimationState {
    fn default() -> Self {
        Self {
            phase_id: default_ender_dragon_phase_id(),
            dragon_dying: false,
            dragon_death_time: 0,
            y_rot_a: 0.0,
            o_flap_time: 0.0,
            flap_time: 0.0,
            no_ai: false,
            flight_history: DragonFlightHistoryState::default(),
            growl_time: None,
        }
    }
}

impl Default for DragonFlightHistoryState {
    fn default() -> Self {
        Self {
            samples: [DragonFlightHistorySample::default(); DRAGON_FLIGHT_HISTORY_LEN],
            head: -1,
        }
    }
}

impl Serialize for DragonFlightHistoryState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("DragonFlightHistoryState", 2)?;
        state.serialize_field("samples", &self.samples.as_slice())?;
        state.serialize_field("head", &self.head)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for DragonFlightHistoryState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DragonFlightHistoryStateWire::deserialize(deserializer)?;
        let mut samples = [DragonFlightHistorySample::default(); DRAGON_FLIGHT_HISTORY_LEN];
        for (sample, value) in samples.iter_mut().zip(wire.samples) {
            *sample = value;
        }
        let head = if (-1..DRAGON_FLIGHT_HISTORY_LEN as i32).contains(&i32::from(wire.head)) {
            wire.head
        } else {
            default_flight_history_head()
        };
        Ok(Self { samples, head })
    }
}

fn default_flight_history_head() -> i8 {
    -1
}

fn default_ender_dragon_phase_id() -> i32 {
    vanilla_entity_metadata_ender_dragon_phase_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
    )
    .expect("generated EnderDragon phase accessor must resolve")
    .1
}

impl EnderDragonAnimationState {
    pub(crate) fn set_phase(&mut self, phase_id: i32) {
        self.phase_id = phase_id;
    }

    pub(crate) fn set_dragon_dying(&mut self, dragon_dying: bool) {
        self.dragon_dying = dragon_dying;
        if !dragon_dying {
            self.clear_dragon_death();
        }
    }

    pub(crate) fn clear_dragon_death(&mut self) {
        self.dragon_death_time = 0;
    }

    pub(crate) fn advance_dragon_death_tick(&mut self, max_ticks: i32) {
        self.dragon_death_time = (self.dragon_death_time + 1).min(max_ticks);
    }

    pub(crate) fn death_time(self, partial_tick: f32) -> f32 {
        if self.dragon_death_time > 0 {
            self.dragon_death_time as f32 + partial_tick
        } else {
            0.0
        }
    }

    /// Vanilla `EnderDragon.aiStep`'s client-side flap/history block: `oFlapTime` saves before the
    /// dead gate; a dead-or-dying dragon advances neither the flap phase nor the history; the flap
    /// rate comes from the delta movement (`0.2 / (horizontal·10 + 1) · 2^Δy`, or a flat `0.1` while
    /// sitting); a no-AI dragon pins `flapTime = 0.5` instead of recording. The halved in-wall rate
    /// cannot occur here: `EnderDragon.inWall` is only ever assigned inside the `ServerLevel` phase
    /// branch, so a remote client always reads its `false` default.
    pub(crate) fn advance_client_tick(
        &mut self,
        transform: EntityTransform,
        is_silent: bool,
        random_seed: &mut u64,
        next_float: fn(&mut u64) -> f32,
        next_int_bound: fn(&mut u64, i32) -> i32,
    ) -> EnderDragonClientSoundTriggers {
        let mut sounds = EnderDragonClientSoundTriggers::default();

        // Vanilla `EnderDragon.aiStep` starts with
        // `processFlappingMovement()`, so the crossing test reads the previous
        // tick's endpoints and the silent gate precedes the entity RNG draw.
        let flap = (self.flap_time * std::f32::consts::TAU).cos();
        let old_flap = (self.o_flap_time * std::f32::consts::TAU).cos();
        if old_flap <= -0.3 && flap >= -0.3 && !is_silent {
            sounds.flap_pitch = Some(0.8 + next_float(random_seed) * 0.3);
        }

        // The short-circuit order is observable: silent or sitting dragons do
        // not decrement the countdown. A firing edge consumes pitch first,
        // then the reset's bounded integer from the same entity RNG.
        if !is_silent && !self.is_sitting() {
            let growl_time = self.growl_time.get_or_insert(100);
            *growl_time -= 1;
            if *growl_time < 0 {
                sounds.growl_pitch = Some(0.8 + next_float(random_seed) * 0.3);
                *growl_time = 200 + next_int_bound(random_seed, 200);
            }
        }

        self.o_flap_time = self.flap_time;
        if self.dragon_dying {
            // Vanilla's death explosion position consumes three floats after
            // the two sound branches. The renderer owns the visual, but this
            // canonical cursor must still preserve later sound ordering.
            next_float(random_seed);
            next_float(random_seed);
            next_float(random_seed);
            return sounds;
        }
        let delta = transform.delta_movement;
        let horizontal = ((delta.x * delta.x + delta.z * delta.z).sqrt()) as f32;
        let flap_speed = 0.2 / (horizontal * 10.0 + 1.0) * (2.0f64.powf(delta.y)) as f32;
        if self.is_sitting() {
            self.flap_time += 0.1;
        } else {
            self.flap_time += flap_speed;
        }
        if self.no_ai {
            self.flap_time = 0.5;
        } else {
            self.flight_history
                .record(transform.position.y, transform.y_rot);
            // The client phase tick runs after flap/history advancement. Unlike
            // `onFlap` and the ambient growl, this phase-local sound has no
            // `isSilent()` gate in Vanilla.
            if self.is_sitting_attacking() {
                sounds.growl_pitch = Some(0.8 + next_float(random_seed) * 0.3);
            }
        }
        sounds
    }

    /// Vanilla `EnderDragonRenderer.extractRenderState`:
    /// `state.flapTime = Mth.lerp(partialTicks, oFlapTime, flapTime)`.
    pub(crate) fn flap_time(self, partial_ticks: f32) -> f32 {
        self.o_flap_time + (self.flap_time - self.o_flap_time) * partial_ticks
    }

    fn is_sitting(self) -> bool {
        vanilla_ender_dragon_phase_is_sitting(self.phase_id)
    }

    fn is_sitting_attacking(self) -> bool {
        usize::try_from(self.phase_id)
            .ok()
            .and_then(|phase_id| VANILLA_ENDER_DRAGON_PHASES_26_1.get(phase_id))
            .is_some_and(|phase| phase.java_field_name == "SITTING_ATTACKING")
    }
}

/// Vanilla `EnderDragonRenderState.isLandingOrTakingOff`:
/// `phase == EnderDragonPhase.LANDING || phase == EnderDragonPhase.TAKEOFF`, resolved through the
/// generated phase catalog by Java field name rather than a hand-written ordinal.
pub(crate) fn vanilla_ender_dragon_phase_is_landing_or_takeoff(phase_id: i32) -> bool {
    usize::try_from(phase_id)
        .ok()
        .and_then(|phase_id| VANILLA_ENDER_DRAGON_PHASES_26_1.get(phase_id))
        .is_some_and(|phase| matches!(phase.java_field_name, "LANDING" | "TAKEOFF"))
}

impl DragonFlightHistoryState {
    fn record(&mut self, y: f64, y_rot: f32) {
        let sample = DragonFlightHistorySample { y, y_rot };
        if self.head < 0 {
            self.samples.fill(sample);
        }

        let next = i32::from(self.head) + 1;
        self.head = if next == DRAGON_FLIGHT_HISTORY_LEN as i32 {
            0
        } else {
            next as i8
        };
        self.samples[self.head as usize] = sample;
    }

    fn get(self, delay: i32) -> DragonFlightHistorySample {
        if self.head < 0 {
            return DragonFlightHistorySample::default();
        }
        let index = (i32::from(self.head) - delay) & DRAGON_FLIGHT_HISTORY_MASK;
        self.samples[index as usize]
    }

    pub(crate) fn get_interpolated(
        self,
        delay: i32,
        partial_ticks: f32,
    ) -> DragonFlightHistorySample {
        let sample = self.get(delay);
        let sample_old = self.get(delay + 1);
        DragonFlightHistorySample {
            y: sample_old.y + (sample.y - sample_old.y) * f64::from(partial_ticks),
            y_rot: sample_old.y_rot + wrap_degrees(sample.y_rot - sample_old.y_rot) * partial_ticks,
        }
    }
}

pub(crate) fn ender_dragon_part_pick_targets_at_partial_tick(
    parent_id: i32,
    transform: EntityTransform,
    animation: Option<EnderDragonAnimationState>,
    partial_ticks: f32,
) -> Vec<EntityPickTargetState> {
    let animation = animation.unwrap_or_default();
    ENDER_DRAGON_PARTS
        .iter()
        .map(|part| EntityPickTargetState {
            entity_id: parent_id + part.id_offset,
            position: part_position(transform, part.offset, animation, partial_ticks),
            bounds: EntityPickBoundsState::from_base_size(part.width, part.height, 0.0),
        })
        .collect()
}

pub(crate) fn ender_dragon_parent_pick_target(
    parent_id: i32,
    transform: EntityTransform,
    partial_ticks: f32,
) -> EntityPickTargetState {
    EntityPickTargetState {
        entity_id: parent_id,
        position: transform.render_position(partial_ticks),
        // Vanilla 26.1 `EntityType.ENDER_DRAGON.sized(16.0F, 8.0F)`.
        bounds: EntityPickBoundsState::from_base_size(16.0, 8.0, 0.0),
    }
}

pub(crate) fn ender_dragon_part_parent_id(
    part_id: i32,
    is_dragon_parent: impl Fn(i32) -> bool,
) -> Option<i32> {
    ENDER_DRAGON_PARTS.iter().find_map(|part| {
        let parent_id = part_id - part.id_offset;
        is_dragon_parent(parent_id).then_some(parent_id)
    })
}

fn part_position(
    transform: EntityTransform,
    offset: EnderDragonPartOffset,
    animation: EnderDragonAnimationState,
    partial_ticks: f32,
) -> EntityVec3 {
    let yaw = f64::from(transform.render_body_y_rot(partial_ticks)).to_radians();
    let sin_yaw = yaw.sin();
    let cos_yaw = yaw.cos();
    let sample_0 = animation.flight_history.get_interpolated(0, partial_ticks);
    let sample_5 = animation.flight_history.get_interpolated(5, partial_ticks);
    let sample_10 = animation.flight_history.get_interpolated(10, partial_ticks);
    let tilt = (sample_5.y - sample_10.y) * 10.0 * std::f64::consts::PI / 180.0;
    let cos_tilt = tilt.cos();
    let sin_tilt = tilt.sin();
    let head_y_offset = if animation.is_sitting() {
        -1.0
    } else {
        sample_5.y - sample_0.y
    };
    let (x, y, z) = match offset {
        EnderDragonPartOffset::Head => head_part_offset(
            yaw,
            animation.y_rot_a,
            cos_tilt,
            sin_tilt,
            head_y_offset,
            6.5,
        ),
        EnderDragonPartOffset::Neck => head_part_offset(
            yaw,
            animation.y_rot_a,
            cos_tilt,
            sin_tilt,
            head_y_offset,
            5.5,
        ),
        EnderDragonPartOffset::Body => (sin_yaw * 0.5, 0.0, -cos_yaw * 0.5),
        EnderDragonPartOffset::Tail(index) => {
            let distance = f64::from(index + 1) * 2.0;
            let p1 = sample_5;
            let p0 = animation
                .flight_history
                .get_interpolated(12 + i32::from(index) * 2, partial_ticks);
            let tail_yaw = yaw + f64::from(wrap_degrees(p0.y_rot - p1.y_rot)).to_radians();
            let tail_sin = tail_yaw.sin();
            let tail_cos = tail_yaw.cos();
            (
                -(sin_yaw * 1.5 + tail_sin * distance) * cos_tilt,
                p0.y - p1.y - (distance + 1.5) * sin_tilt + 1.5,
                (cos_yaw * 1.5 + tail_cos * distance) * cos_tilt,
            )
        }
        EnderDragonPartOffset::Wing(side) => (cos_yaw * 4.5 * side, 2.0, sin_yaw * 4.5 * side),
    };
    EntityVec3 {
        x: transform.render_position(partial_ticks).x + x,
        y: transform.render_position(partial_ticks).y + y,
        z: transform.render_position(partial_ticks).z + z,
    }
}

fn head_part_offset(
    yaw: f64,
    y_rot_a: f32,
    cos_tilt: f64,
    sin_tilt: f64,
    head_y_offset: f64,
    distance: f64,
) -> (f64, f64, f64) {
    let head_yaw = yaw - f64::from(y_rot_a) * 0.01;
    (
        head_yaw.sin() * distance * cos_tilt,
        head_y_offset + sin_tilt * distance,
        -head_yaw.cos() * distance * cos_tilt,
    )
}

fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::schema::VANILLA_ENDER_DRAGON_PHASES_26_1;

    #[test]
    fn dragon_flight_history_interpolates_like_vanilla() {
        let mut history = DragonFlightHistoryState::default();
        history.record(64.0, 170.0);
        history.record(74.0, -170.0);

        assert_eq!(
            history.get_interpolated(0, 0.0),
            DragonFlightHistorySample {
                y: 64.0,
                y_rot: 170.0,
            }
        );
        assert_eq!(
            history.get_interpolated(0, 0.5),
            DragonFlightHistorySample {
                y: 69.0,
                y_rot: 180.0,
            }
        );
        assert_eq!(
            history.get_interpolated(0, 1.0),
            DragonFlightHistorySample {
                y: 74.0,
                y_rot: 190.0,
            }
        );
    }

    #[test]
    fn dragon_sitting_pose_matches_generated_phase_class_behavior() {
        for phase in VANILLA_ENDER_DRAGON_PHASES_26_1 {
            let animation = EnderDragonAnimationState {
                phase_id: phase.phase_id,
                ..EnderDragonAnimationState::default()
            };
            assert_eq!(
                animation.is_sitting(),
                phase.is_sitting,
                "{} ({})",
                phase.java_field_name,
                phase.implementation_class
            );
        }

        let hover = VANILLA_ENDER_DRAGON_PHASES_26_1
            .iter()
            .find(|phase| phase.implementation_class.ends_with(".DragonHoverPhase"))
            .expect("generated DragonHoverPhase registration");
        assert!(
            hover.is_sitting,
            "DragonHoverPhase directly overrides isSitting"
        );
        assert!(!EnderDragonAnimationState {
            phase_id: -1,
            ..EnderDragonAnimationState::default()
        }
        .is_sitting());
        assert!(!EnderDragonAnimationState {
            phase_id: VANILLA_ENDER_DRAGON_PHASES_26_1.len() as i32,
            ..EnderDragonAnimationState::default()
        }
        .is_sitting());
    }

    fn transform_at(y: f64, y_rot: f32, delta: [f64; 3]) -> EntityTransform {
        let position = EntityVec3 { x: 0.0, y, z: 0.0 };
        EntityTransform {
            position,
            old_position: position,
            position_base: position,
            delta_movement: EntityVec3 {
                x: delta[0],
                y: delta[1],
                z: delta[2],
            },
            y_rot,
            old_y_rot: y_rot,
            x_rot: 0.0,
            old_x_rot: 0.0,
            y_head_rot: y_rot,
            old_y_head_rot: y_rot,
            on_ground: None,
            interpolation: None,
        }
    }

    fn advance(dragon: &mut EnderDragonAnimationState, transform: EntityTransform) {
        fn next_float(_: &mut u64) -> f32 {
            0.5
        }
        fn next_int_bound(_: &mut u64, _: i32) -> i32 {
            0
        }
        let mut seed = 0;
        dragon.advance_client_tick(transform, false, &mut seed, next_float, next_int_bound);
    }

    #[test]
    fn dragon_client_flap_advance_matches_vanilla_ai_step() {
        // Vanilla `EnderDragon.aiStep`: `flapSpeed = 0.2 / ((float)horizontalDistance · 10 + 1) ·
        // (float)2^Δy`, added to `flapTime` after `oFlapTime` saves the previous phase. The default
        // client phase is HOVERING, whose `DragonHoverPhase.isSitting()` is true, so the
        // movement-derived branch needs an explicitly non-sitting phase.
        let flying = VANILLA_ENDER_DRAGON_PHASES_26_1
            .iter()
            .find(|phase| !phase.is_sitting)
            .expect("generated non-sitting phase");
        let mut dragon = EnderDragonAnimationState {
            phase_id: flying.phase_id,
            ..EnderDragonAnimationState::default()
        };
        advance(&mut dragon, transform_at(64.0, 30.0, [0.3, 0.1, 0.4]));
        let expected = 0.2 / (0.5_f32 * 10.0 + 1.0) * (2.0_f64.powf(0.1)) as f32;
        assert_eq!(dragon.o_flap_time, 0.0);
        assert!((dragon.flap_time - expected).abs() < 1.0e-7, "{dragon:?}");
        assert_eq!(
            dragon.flight_history.get(0),
            DragonFlightHistorySample {
                y: 64.0,
                y_rot: 30.0,
            }
        );
        // `Mth.lerp(partial, oFlapTime, flapTime)` for the render-state extraction.
        assert!((dragon.flap_time(0.5) - expected * 0.5).abs() < 1.0e-7);

        // A sitting phase replaces the movement-derived rate with a flat `+0.1`.
        let sitting = VANILLA_ENDER_DRAGON_PHASES_26_1
            .iter()
            .find(|phase| phase.is_sitting)
            .expect("generated sitting phase");
        let mut sitting_dragon = EnderDragonAnimationState {
            phase_id: sitting.phase_id,
            ..EnderDragonAnimationState::default()
        };
        advance(
            &mut sitting_dragon,
            transform_at(64.0, 0.0, [0.3, 0.1, 0.4]),
        );
        assert!((sitting_dragon.flap_time - 0.1).abs() < 1.0e-7);

        // A no-AI dragon pins `flapTime = 0.5` and records no flight history.
        let mut no_ai = EnderDragonAnimationState {
            no_ai: true,
            ..EnderDragonAnimationState::default()
        };
        advance(&mut no_ai, transform_at(70.0, 45.0, [0.0, 0.0, 0.0]));
        assert_eq!(no_ai.flap_time, 0.5);
        assert_eq!(no_ai.flight_history.head, -1);

        // A dead-or-dying dragon saves `oFlapTime` but advances neither flap nor history.
        let mut dying = EnderDragonAnimationState {
            dragon_dying: true,
            flap_time: 0.75,
            o_flap_time: 0.5,
            ..EnderDragonAnimationState::default()
        };
        advance(&mut dying, transform_at(70.0, 45.0, [0.0, 0.2, 0.0]));
        assert_eq!((dying.o_flap_time, dying.flap_time), (0.75, 0.75));
        assert_eq!(dying.flight_history.head, -1);
    }

    #[test]
    fn dragon_landing_or_takeoff_phases_match_generated_catalog() {
        for phase in VANILLA_ENDER_DRAGON_PHASES_26_1 {
            assert_eq!(
                vanilla_ender_dragon_phase_is_landing_or_takeoff(phase.phase_id),
                matches!(phase.java_field_name, "LANDING" | "TAKEOFF"),
                "{}",
                phase.java_field_name
            );
        }
        // Pin the vanilla registration ordinals so catalog drift is loud.
        assert!(vanilla_ender_dragon_phase_is_landing_or_takeoff(3));
        assert!(vanilla_ender_dragon_phase_is_landing_or_takeoff(4));
        assert!(!vanilla_ender_dragon_phase_is_landing_or_takeoff(0));
        assert!(!vanilla_ender_dragon_phase_is_landing_or_takeoff(-1));
    }
}
