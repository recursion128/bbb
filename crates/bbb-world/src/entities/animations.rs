use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

use super::dragon::{
    EnderDragonAnimationState, ENDER_DRAGON_PHASE_DATA_ID, ENDER_DRAGON_PHASE_HOVERING_ID,
    VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
};
use super::EntityTransform;

const VANILLA_ENTITY_TYPE_POLAR_BEAR_ID: i32 = 104;
const VANILLA_ENTITY_TYPE_SHULKER_ID: i32 = 112;
const POLAR_BEAR_STANDING_DATA_ID: u8 = 18;
const POLAR_BEAR_STAND_ANIMATION_TICKS: f32 = 6.0;
const SHULKER_PEEK_DATA_ID: u8 = 17;
const SHULKER_PEEK_PER_TICK: f32 = 0.05;
const SHULKER_MAX_PEEK_AMOUNT: f32 = 1.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct EntityClientAnimationState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub polar_bear_standing: Option<PolarBearStandingAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shulker_peek: Option<ShulkerPeekAnimationState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ender_dragon: Option<EnderDragonAnimationState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolarBearStandingAnimationState {
    pub target_standing: bool,
    pub previous_ticks: f32,
    pub current_ticks: f32,
    pub dimensions_ticks: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShulkerPeekAnimationState {
    pub target_peek_amount: f32,
    pub previous_peek_amount: f32,
    pub current_peek_amount: f32,
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

impl Default for ShulkerPeekAnimationState {
    fn default() -> Self {
        Self {
            target_peek_amount: 0.0,
            previous_peek_amount: 0.0,
            current_peek_amount: 0.0,
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

impl ShulkerPeekAnimationState {
    fn set_target(&mut self, target_peek_amount: f32) {
        self.target_peek_amount = target_peek_amount;
    }

    fn advance_client_tick(&mut self) {
        self.previous_peek_amount = self.current_peek_amount;
        if self.current_peek_amount == self.target_peek_amount {
            return;
        }

        self.current_peek_amount = if self.current_peek_amount > self.target_peek_amount {
            (self.current_peek_amount - SHULKER_PEEK_PER_TICK)
                .clamp(self.target_peek_amount, SHULKER_MAX_PEEK_AMOUNT)
        } else {
            (self.current_peek_amount + SHULKER_PEEK_PER_TICK).clamp(0.0, self.target_peek_amount)
        };
    }
}

impl EntityClientAnimationState {
    pub(crate) fn sync_targets_from_metadata(
        &mut self,
        entity_type_id: i32,
        data_values: &[EntityDataValue],
    ) {
        match entity_type_id {
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                let target_standing =
                    entity_data_bool(data_values, POLAR_BEAR_STANDING_DATA_ID, false);
                if let Some(standing) = self.polar_bear_standing.as_mut() {
                    standing.set_target(target_standing);
                } else if target_standing {
                    self.polar_bear_standing = Some(PolarBearStandingAnimationState {
                        target_standing,
                        ..PolarBearStandingAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                let target_peek_amount = shulker_target_peek_amount(data_values);
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.set_target(target_peek_amount);
                } else if target_peek_amount > 0.0 {
                    self.shulker_peek = Some(ShulkerPeekAnimationState {
                        target_peek_amount,
                        ..ShulkerPeekAnimationState::default()
                    });
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => {
                let phase_id = entity_data_int(
                    data_values,
                    ENDER_DRAGON_PHASE_DATA_ID,
                    ENDER_DRAGON_PHASE_HOVERING_ID,
                );
                if let Some(dragon) = self.ender_dragon.as_mut() {
                    dragon.set_phase(phase_id);
                } else {
                    self.ender_dragon = Some(EnderDragonAnimationState {
                        phase_id,
                        ..EnderDragonAnimationState::default()
                    });
                }
            }
            _ => {}
        }
    }

    pub(crate) fn advance_client_tick(&mut self, entity_type_id: i32, transform: EntityTransform) {
        match entity_type_id {
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => {
                if let Some(standing) = self.polar_bear_standing.as_mut() {
                    standing.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_SHULKER_ID => {
                if let Some(peek) = self.shulker_peek.as_mut() {
                    peek.advance_client_tick();
                }
            }
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID => self
                .ender_dragon
                .get_or_insert_with(EnderDragonAnimationState::default)
                .advance_client_tick(transform),
            _ => {}
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

fn shulker_target_peek_amount(data_values: &[EntityDataValue]) -> f32 {
    f32::from(entity_data_byte(data_values, SHULKER_PEEK_DATA_ID, 0).clamp(0, 100)) * 0.01
}

fn entity_data_byte(data_values: &[EntityDataValue], data_id: u8, default: i8) -> i8 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Byte(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(default)
}

fn entity_data_int(data_values: &[EntityDataValue], data_id: u8, fallback: i32) -> i32 {
    data_values
        .iter()
        .find(|value| value.data_id == data_id)
        .and_then(|value| match &value.value {
            EntityDataValueKind::Int(value) => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}
