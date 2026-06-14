use uuid::Uuid;

use super::{EntityState, EntityVec3};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityIdentity {
    pub(crate) id: i32,
    pub(crate) uuid: Uuid,
    pub(crate) entity_type_id: i32,
    pub(crate) data: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EntityTransform {
    pub(crate) position: EntityVec3,
    pub(crate) position_base: EntityVec3,
    pub(crate) delta_movement: EntityVec3,
    pub(crate) y_rot: f32,
    pub(crate) x_rot: f32,
    pub(crate) y_head_rot: f32,
    pub(crate) on_ground: Option<bool>,
}

impl From<&EntityState> for EntityIdentity {
    fn from(state: &EntityState) -> Self {
        Self {
            id: state.id,
            uuid: state.uuid,
            entity_type_id: state.entity_type_id,
            data: state.data,
        }
    }
}

impl From<&EntityState> for EntityTransform {
    fn from(state: &EntityState) -> Self {
        Self {
            position: state.position,
            position_base: state.position_base,
            delta_movement: state.delta_movement,
            y_rot: state.y_rot,
            x_rot: state.x_rot,
            y_head_rot: state.y_head_rot,
            on_ground: state.on_ground,
        }
    }
}

impl EntityTransform {
    pub(crate) fn write_to_state(self, state: &mut EntityState) {
        state.position = self.position;
        state.position_base = self.position_base;
        state.delta_movement = self.delta_movement;
        state.y_rot = self.y_rot;
        state.x_rot = self.x_rot;
        state.y_head_rot = self.y_head_rot;
        state.on_ground = self.on_ground;
    }
}
