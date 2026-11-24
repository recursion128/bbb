//! Base mob body/head rotation shared by client presentation and server movement.

use crate::entity_types::{
    VANILLA_ENTITY_TYPE_ARMADILLO_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    VANILLA_ENTITY_TYPE_BREEZE_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_FROG_ID,
    VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_PHANTOM_ID, VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
    VANILLA_ENTITY_TYPE_SNIFFER_ID,
};
use crate::packets::Vec3d;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyRotationControl {
    max_head_y_rot: f32,
    head_stable_time: i32,
    last_stable_y_head_rot: f32,
}

impl BodyRotationControl {
    pub fn for_entity(entity_type_id: i32) -> Option<Self> {
        Some(Self {
            max_head_y_rot: base_body_rotation_max_yaw(entity_type_id)?,
            head_stable_time: 0,
            last_stable_y_head_rot: 0.0,
        })
    }

    // Vanilla `BodyRotationControl.clientTick` (`BodyRotationControl.java:6-63`).
    pub fn tick(
        &mut self,
        previous_position: Vec3d,
        position: Vec3d,
        y_rot: f32,
        y_body_rot: &mut f32,
        y_head_rot: &mut f32,
        carrying_mob_passenger: bool,
    ) {
        let xd = position.x - previous_position.x;
        let zd = position.z - previous_position.z;
        if xd * xd + zd * zd > f64::from(2.500_000_3e-7_f32) {
            *y_body_rot = y_rot;
            *y_head_rot = rotate_if_necessary(*y_head_rot, *y_body_rot, self.max_head_y_rot);
            self.last_stable_y_head_rot = *y_head_rot;
            self.head_stable_time = 0;
        } else if !carrying_mob_passenger {
            if (*y_head_rot - self.last_stable_y_head_rot).abs() > 15.0 {
                self.head_stable_time = 0;
                self.last_stable_y_head_rot = *y_head_rot;
                *y_body_rot = rotate_if_necessary(*y_body_rot, *y_head_rot, self.max_head_y_rot);
            } else {
                self.head_stable_time = self.head_stable_time.wrapping_add(1);
                if self.head_stable_time > 10 {
                    let fraction = ((self.head_stable_time - 10) as f32 / 10.0).clamp(0.0, 1.0);
                    *y_body_rot = rotate_if_necessary(
                        *y_body_rot,
                        *y_head_rot,
                        self.max_head_y_rot * (1.0 - fraction),
                    );
                }
            }
        }
    }
}

// Vanilla `Mth.rotateIfNecessary` (`Mth.java:249-253`).
fn rotate_if_necessary(base: f32, target: f32, max_difference: f32) -> f32 {
    let mut difference = (target - base) % 360.0;
    if difference >= 180.0 {
        difference -= 360.0;
    }
    if difference < -180.0 {
        difference += 360.0;
    }
    target - difference.clamp(-max_difference, max_difference)
}

pub const fn base_body_rotation_max_yaw(entity_type_id: i32) -> Option<f32> {
    match entity_type_id {
        // Custom controllers own additional subtype state: `Phantom.java:65-67`,
        // `Shulker.java:150-152`, `Creaking.java:109-111`, `HappyGhast.java:562-564`,
        // `Camel.java:657-668`, `CamelHusk.java:24-25`, `Armadillo.java:365-379`.
        VANILLA_ENTITY_TYPE_PHANTOM_ID
        | VANILLA_ENTITY_TYPE_SHULKER_ID
        | VANILLA_ENTITY_TYPE_CREAKING_ID
        | VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
        | VANILLA_ENTITY_TYPE_CAMEL_ID
        | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        | VANILLA_ENTITY_TYPE_ARMADILLO_ID => None,
        // Vanilla `getMaxHeadYRot` overrides: `Ravager.java:128-130`, `Breeze.java:249-251`,
        // `Goat.java:180-182`, `Dolphin.java:200-202`, `Sniffer.java:417-419`,
        // `Axolotl.java:419-421`, `Frog.java:130-132`.
        VANILLA_ENTITY_TYPE_RAVAGER_ID => Some(45.0),
        VANILLA_ENTITY_TYPE_BREEZE_ID => Some(30.0),
        VANILLA_ENTITY_TYPE_GOAT_ID => Some(15.0),
        VANILLA_ENTITY_TYPE_DOLPHIN_ID | VANILLA_ENTITY_TYPE_AXOLOTL_ID => Some(1.0),
        VANILLA_ENTITY_TYPE_SNIFFER_ID => Some(50.0),
        VANILLA_ENTITY_TYPE_FROG_ID => Some(5.0),
        // Vanilla `Mob.getMaxHeadYRot` (`Mob.java:729-731`).
        _ => Some(75.0),
    }
}
