use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const VANILLA_ENTITY_TYPE_POLAR_BEAR_ID: i32 = 104;
const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;
const POLAR_BEAR_STAND_ANIMATION_TICKS: f32 = 6.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polar_bear_standing: Option<PolarBearStandingAnimationState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolarBearStandingAnimationState {
    pub target_standing: bool,
    pub previous_ticks: f32,
    pub current_ticks: f32,
    pub dimensions_ticks: f32,
}

impl Default for PolarBearStandingAnimationState {
    fn default() -> Self {
        Self {
            target_standing: false,
            previous_ticks: 0.0,
            current_ticks: 0.0,
            dimensions_ticks: 0.0,
        }
    }
}

impl PolarBearStandingAnimationState {
    pub(crate) fn dimensions_height_scale(self) -> f32 {
        1.0 + self.dimensions_ticks / POLAR_BEAR_STAND_ANIMATION_TICKS
    }

    fn set_target(&mut self, target_standing: bool) {
        self.target_standing = target_standing;
    }

    fn advance_client_tick(&mut self) {
        if self.current_ticks != self.previous_ticks {
            self.dimensions_ticks = self.current_ticks;
        }

        self.previous_ticks = self.current_ticks;
        self.current_ticks = if self.target_standing {
            (self.current_ticks + 1.0).min(POLAR_BEAR_STAND_ANIMATION_TICKS)
        } else {
            (self.current_ticks - 1.0).max(0.0)
        };
    }
}

impl EntityClientAnimationState {
    pub(crate) fn sync_targets_from_metadata(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
    ) {
        if entity_type_id != VANILLA_ENTITY_TYPE_POLAR_BEAR_ID {
            return;
        }

        let target_standing = entity_data_bool(data_values, POLAR_BEAR_STANDING_DATA_ID, false);
        if let Some(standing) = self.polar_bear_standing.as_mut() {
            standing.set_target(target_standing);
        } else if target_standing {
            self.polar_bear_standing = Some(PolarBearStandingAnimationState {
                target_standing,
                ..PolarBearStandingAnimationState::default()
            });
        }
    }

    pub(crate) fn advance_client_tick(&mut self, entity_type_id: i32) {
        if entity_type_id != VANILLA_ENTITY_TYPE_POLAR_BEAR_ID {
            return;
        }

        if let Some(standing) = self.polar_bear_standing.as_mut() {
            standing.advance_client_tick();
        }
    }
}

impl WorldStore {
    pub fn advance_entity_client_animations(&mut self, ticks: u32) {
        self.entities.advance_client_animations(ticks);
    }
}

fn entity_data_bool(data_values: &[EntityDataValue], data_id: u8, default: bool) -> bool {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Boolean(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}
