use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::fluid::block_state_is_air;

use crate::entity::living::wrap_degrees;
use crate::entity::save::optional_byte;
use crate::entity::{Entity, EntityData};
use crate::world::simulation_level::LevelCtx;

use super::MobAiContext;

pub(crate) const AMBIENT_VOLUME: f32 = 0.1;

impl super::MobAiRuntime {
    pub(crate) fn bat_ambient_pitch(&mut self, resting: bool) -> Option<f32> {
        if resting && self.random.next_int(4) != 0 {
            return None;
        }
        Some(self.voice_pitch(false) * 0.95)
    }

    pub(crate) fn tick_bat_ambient_sound(&mut self, alive: bool, resting: bool) -> Option<f32> {
        if self.tick_mob_ambient_sound(alive) {
            self.bat_ambient_pitch(resting)
        } else {
            None
        }
    }
}

// `Bat` owns one tracked flag byte and preserves bits beyond the resting flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatRetainedState {
    flags: i8,
}

impl BatRetainedState {
    pub(crate) const fn from_saved(flags: i8) -> Self {
        Self { flags }
    }

    pub(crate) const fn flags(self) -> i8 {
        self.flags
    }

    pub(crate) const fn is_resting(self) -> bool {
        self.flags & 1 != 0
    }

    pub(crate) fn set_resting(&mut self, resting: bool) -> bool {
        let next = if resting {
            self.flags | 1
        } else {
            self.flags & !1
        };
        let changed = next != self.flags;
        self.flags = next;
        changed
    }
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: BatRetainedState) {
    // `Bat.addAdditionalSaveData` (`Bat.java:217-220`).
    entries.push(NbtSummaryEntry {
        name: "BatFlags".into(),
        value: NbtSummaryValue::Byte(state.flags()),
    });
}

pub(crate) fn read_save_data(entries: &[NbtSummaryEntry]) -> BatRetainedState {
    // `Bat.readAdditionalSaveData` defaults a missing byte to zero
    BatRetainedState::from_saved(optional_byte(entries, "BatFlags").unwrap_or(0))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatRuntime {
    target_position: Option<BlockPos>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct BatAiStepResult {
    pub(crate) resting: Option<bool>,
    pub(crate) delta_movement: Option<Vec3d>,
    pub(crate) y_rot: Option<f32>,
    pub(crate) takeoff_event: bool,
}

impl BatRuntime {
    pub(crate) const fn new() -> Self {
        Self {
            target_position: None,
        }
    }

    // `Bat.customServerAiStep`.
    pub(crate) fn custom_server_ai_step(
        &mut self,
        context: &mut MobAiContext,
        entity: &Entity,
        level: &LevelCtx<'_, '_>,
    ) -> BatAiStepResult {
        let EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!("Bat runtime belongs to a general living entity");
        };
        let bat = data.bat_retained().expect("Bat runtime has retained state");
        let above = BlockPos {
            y: entity.block_position().y + 1,
            ..entity.block_position()
        };
        let content_registry = level.content_registry();
        let above_state = level.block_state(above);
        let target_state = self.target_position.map(|target| level.block_state(target));
        let known_block = |state: i32| {
            vanilla_block_state_block_type_id(state).is_some()
                || content_registry.block_by_state_id(state).is_some()
        };
        if !known_block(above_state) || target_state.is_some_and(|state| !known_block(state)) {
            return BatAiStepResult::default();
        }
        let above_is_conductor =
            crate::redstone::is_redstone_conductor_with_registry(above_state, content_registry);
        if bat.is_resting() {
            if above_is_conductor {
                if context.next_int(200) == 0 {
                    context.body.y_head_rot = context.next_int(360) as f32;
                }
                if !level
                    .nearest_player(entity.position(), 4.0)
                    .is_some_and(|player| !player.spectator && !player.dead_or_dying)
                {
                    return BatAiStepResult::default();
                }
            }
            return BatAiStepResult {
                resting: Some(false),
                takeoff_event: true,
                ..BatAiStepResult::default()
            };
        }

        let mut target = self.target_position;
        if target_state.is_some_and(|state| !block_state_is_air(state))
            || target.is_some_and(|target| target.y <= level.min_y())
        {
            target = None;
        }
        let position = entity.position();
        let choose_target = match target {
            None => true,
            Some(target) => context.next_int(30) == 0 || target_is_close(target, position),
        };
        if choose_target {
            target = Some(select_target(position, context));
        }
        let target = target.expect("Bat always selects a target before steering");
        self.target_position = Some(target);

        let dx = f64::from(target.x) + 0.5 - position.x;
        let dy = f64::from(target.y) + 0.1 - position.y;
        let dz = f64::from(target.z) + 0.5 - position.z;
        let scale = f64::from(0.1_f32);
        let delta_movement = entity.delta_movement();
        let movement = Vec3d {
            x: delta_movement.x + (java_signum(dx) * 0.5 - delta_movement.x) * scale,
            y: delta_movement.y + (java_signum(dy) * 0.7 - delta_movement.y) * scale,
            z: delta_movement.z + (java_signum(dz) * 0.5 - delta_movement.z) * scale,
        };
        let target_y_rot = (movement.z.atan2(movement.x) * f64::from(180.0_f32)
            / f64::from(std::f32::consts::PI)) as f32
            - 90.0;
        let y_rot = entity.y_rot() + wrap_degrees(target_y_rot - entity.y_rot());
        context.controls.zza = 0.5;
        let rests = context.next_int(100) == 0 && above_is_conductor;
        BatAiStepResult {
            resting: rests.then_some(true),
            delta_movement: Some(movement),
            y_rot: Some(y_rot),
            takeoff_event: false,
        }
    }
}

fn java_signum(value: f64) -> f64 {
    if value > 0.0 {
        1.0
    } else if value < 0.0 {
        -1.0
    } else {
        value
    }
}

fn target_is_close(target: BlockPos, position: Vec3d) -> bool {
    let dx = f64::from(target.x) + 0.5 - position.x;
    let dy = f64::from(target.y) + 0.5 - position.y;
    let dz = f64::from(target.z) + 0.5 - position.z;
    dx * dx + dy * dy + dz * dz < 4.0
}

fn select_target(position: Vec3d, context: &mut MobAiContext) -> BlockPos {
    BlockPos {
        x: (position.x + f64::from(context.next_int(7) - context.next_int(7))).floor() as i32,
        y: (position.y + f64::from(context.next_int(6)) - 2.0).floor() as i32,
        z: (position.z + f64::from(context.next_int(7) - context.next_int(7))).floor() as i32,
    }
}

#[cfg(test)]
mod tests;
