//! Persistent base-mob body yaw, independent of synchronized entity/head rotations.

use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::entity_body_rotation::BodyRotationControl;
use bbb_protocol::packets::Vec3d;
use serde::{Deserialize, Serialize};

use super::{EntityClientAnimationState, EntityTransform};
use crate::entities::content_facts::entity_is_mob;
use crate::entities::record::rot_lerp;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct MobBodyRotationState {
    previous_y_rot: f32,
    y_rot: f32,
    control: BodyRotationControl,
}

impl MobBodyRotationState {
    pub(crate) fn from_spawn_with_registry(
        entity_type_id: i32,
        head_y_rot: f32,
        content_registry: &ContentRegistry,
    ) -> Option<Self> {
        if !entity_is_mob(entity_type_id, content_registry) {
            return None;
        }
        Some(Self {
            previous_y_rot: head_y_rot,
            y_rot: head_y_rot,
            control: BodyRotationControl::for_entity(entity_type_id)?,
        })
    }
}

impl EntityClientAnimationState {
    pub(crate) fn begin_body_rotation_tick(&mut self) {
        if let Some(body) = self.base_mob_body_rotation.as_mut() {
            body.previous_y_rot = body.y_rot;
        }
    }

    pub(crate) fn set_ridden_body_y_rot(&mut self, y_rot: f32) {
        if let Some(body) = self.base_mob_body_rotation.as_mut() {
            body.y_rot = y_rot;
        }
    }

    pub(crate) fn render_body_y_rot(&self, transform: EntityTransform, partial_tick: f32) -> f32 {
        if let Some(body) = self.base_mob_body_rotation {
            rot_lerp(partial_tick, body.previous_y_rot, body.y_rot)
        } else if let Some(y_rot) = self.spawn_body_y_rot {
            y_rot
        } else {
            transform.render_body_y_rot(partial_tick)
        }
    }

    pub(crate) fn advance_body_rotation_tick(
        &mut self,
        entity_type_id: i32,
        transform: &mut EntityTransform,
        carrying_mob_passenger: bool,
    ) {
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_ENDERMITE_ID, VANILLA_ENTITY_TYPE_SILVERFISH_ID,
        };
        let Some(body) = self.base_mob_body_rotation.as_mut() else {
            return;
        };
        // These tick overrides seed the body before the ordinary mob controller.
        // Vanilla `Endermite.tick` (`Endermite.java:93-96`), `Silverfish.tick` (`Silverfish.java:96-99`).
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_ENDERMITE_ID | VANILLA_ENTITY_TYPE_SILVERFISH_ID
        ) {
            body.previous_y_rot = transform.old_y_rot;
            body.y_rot = transform.y_rot;
        }
        body.control.tick(
            Vec3d {
                x: transform.old_position.x,
                y: transform.old_position.y,
                z: transform.old_position.z,
            },
            Vec3d {
                x: transform.position.x,
                y: transform.position.y,
                z: transform.position.z,
            },
            transform.y_rot,
            &mut body.y_rot,
            &mut transform.y_head_rot,
            carrying_mob_passenger,
        );
    }
}
