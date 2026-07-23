use bbb_protocol::entity_types::{
    vanilla_entity_type_is_fire_immune, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_POLAR_BEAR_ID, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
    VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
    VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
};
use bbb_protocol::packets::{
    EntityAnimation as ProtocolEntityAnimation, EntityEvent as ProtocolEntityEvent,
    HurtAnimation as ProtocolHurtAnimation, RotateHead as ProtocolRotateHead,
    SetEntityLink as ProtocolSetEntityLink, SetEntityMotion as ProtocolSetEntityMotion,
};

use crate::bed_blocks::BedModelFacing;
use crate::client::entity_type_safe_dismount_geometry;
use crate::terrain::{block_name_is_bed, block_name_is_campfire};
use crate::{BlockPos, BlockProbe, WorldStore};

use super::movement::entity_vec3;
use super::schema::{
    vanilla_entity_metadata_type_is_leashable, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_uses_player_entity_event_handler,
};
use super::{
    vanilla_entity_event_kind, EntityVec3, LivingDeathWakeTransition, VanillaEntityEventKind,
    VANILLA_ENTITY_TYPE_PLAYER_ID,
};

/// Vanilla `ClientboundAnimatePacket.SWING_MAIN_HAND` action id: the entity swings its main hand.
const SWING_MAIN_HAND_ACTION: u8 = 0;
/// Vanilla `ClientboundAnimatePacket.WAKE_UP` action id.
const PLAYER_WAKE_UP_ACTION: u8 = 2;
/// Vanilla `ClientboundAnimatePacket.SWING_OFF_HAND` action id: the entity swings its off hand.
const SWING_OFF_HAND_ACTION: u8 = 3;
impl WorldStore {
    pub fn apply_entity_animation(&mut self, packet: ProtocolEntityAnimation) -> bool {
        self.counters.entity_animation_updates_received += 1;
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.id, |events| {
                events.last_animation_action = Some(packet.action)
            })
        else {
            self.counters.entity_animation_updates_ignored += 1;
            return false;
        };
        // Vanilla `ClientboundAnimatePacket`: action `0` swings the main hand, `3` the off hand
        // (`ClientPacketListener.handleAnimate` → `LivingEntity.swing`). Both arm the melee swing.
        if packet.action == SWING_MAIN_HAND_ACTION || packet.action == SWING_OFF_HAND_ACTION {
            if self
                .entities
                .entity_type_id(packet.id)
                .is_some_and(vanilla_entity_metadata_type_is_living)
            {
                let off_hand = packet.action == SWING_OFF_HAND_ACTION;
                let duration = self.entity_held_item_swing_duration(packet.id, off_hand);
                let _ = self
                    .entities
                    .trigger_client_animation_swing(packet.id, off_hand, duration);
            }
        } else if packet.action == PLAYER_WAKE_UP_ACTION
            && self.entities.entity_type_id(packet.id) == Some(VANILLA_ENTITY_TYPE_PLAYER_ID)
        {
            let wake = self.sleeping_bed_wake_transition(packet.id);
            if self
                .entities
                .apply_client_player_wake_animation(packet.id, wake)
                .is_some()
                && self.local_player_id() == Some(packet.id)
            {
                if let (Some(transform), Some(mut pose)) =
                    (self.entities.transform(packet.id), self.local_player.pose)
                {
                    pose.position = bbb_protocol::packets::Vec3d {
                        x: transform.position.x,
                        y: transform.position.y,
                        z: transform.position.z,
                    };
                    pose.y_rot = transform.y_rot;
                    pose.x_rot = transform.x_rot;
                    pose.sneaking = false;
                    pose.swimming = false;
                    self.local_player.pose = Some(pose);
                    self.local_player_old_pose = Some(pose);
                }
            }
        }
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let permission_applied =
            self.apply_local_player_permission_entity_event(packet.entity_id, packet.event_id);
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.entity_id, |events| {
                events.last_event_id = Some(packet.event_id)
            })
        else {
            if permission_applied {
                self.counters.entity_events_applied += 1;
                return true;
            }
            self.counters.entity_events_ignored += 1;
            return false;
        };
        let event_kind = self
            .entities
            .entity_type_id(packet.entity_id)
            .and_then(|entity_type_id| vanilla_entity_event_kind(entity_type_id, packet.event_id));
        let _ = self
            .entities
            .apply_client_animation_entity_event(packet.entity_id, packet.event_id);
        if event_kind == Some(VanillaEntityEventKind::LivingDeath) {
            let wake = self.living_death_wake_transition(packet.entity_id);
            let _ = self
                .entities
                .apply_client_living_death_entity_event(packet.entity_id, wake);
        }
        if self
            .entities
            .entity_type_id(packet.entity_id)
            .is_some_and(vanilla_entity_metadata_type_uses_player_entity_event_handler)
        {
            match event_kind {
                Some(VanillaEntityEventKind::PlayerFinishUsingItem)
                    if self.local_player_id() == Some(packet.entity_id) =>
                {
                    // `ServerPlayer.completeUsingItem` sends event 9 only to the completing
                    // player's connection. The authoritative stack/effect changes arrive through
                    // their own packets; this event closes the local use/movement/render gates.
                    let _ = self.take_local_using_item();
                }
                Some(VanillaEntityEventKind::PlayerEnableReducedDebug) => {
                    let _ = self.set_local_player_reduced_debug_info_from_entity_event(
                        packet.entity_id,
                        true,
                    );
                }
                Some(VanillaEntityEventKind::PlayerDisableReducedDebug) => {
                    let _ = self.set_local_player_reduced_debug_info_from_entity_event(
                        packet.entity_id,
                        false,
                    );
                }
                _ => {}
            }
        }
        if event_kind == Some(VanillaEntityEventKind::LivingSwapHandItems) {
            let _ = self.apply_living_swap_hand_items_event(packet.entity_id);
        }
        self.counters.entity_events_applied += 1;
        true
    }

    fn living_death_wake_transition(
        &mut self,
        entity_id: i32,
    ) -> Option<LivingDeathWakeTransition> {
        if self.entities.living_entity_dead(entity_id) != Some(false) {
            return None;
        }
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        if vanilla_entity_metadata_type_uses_player_entity_event_handler(entity_type_id) {
            return None;
        }
        self.sleeping_bed_wake_transition(entity_id)
    }

    fn sleeping_bed_wake_transition(
        &mut self,
        entity_id: i32,
    ) -> Option<LivingDeathWakeTransition> {
        let entity_type_id = self.entities.entity_type_id(entity_id)?;
        let bed_pos = self.entities.sleeping_pos(entity_id)?;
        let state = self.entities.get(entity_id)?;
        let bed = self.probe_block(bed_pos)?;
        let block_name = bed.block_name.clone()?;
        if !block_name_is_bed(&block_name) {
            return None;
        }
        let facing = BedModelFacing::parse(bed.block_properties.get("facing")?)?;

        // `LivingEntity.stopSleeping` clears the loaded bed's OCCUPIED
        // property before searching for the stand-up position.
        let mut properties = bed.block_properties.clone();
        properties.insert("occupied".to_string(), "false".to_string());
        if let Some(block_state_id) = self
            .registries
            .block_state_id_by_name_and_properties(&block_name, &properties)
        {
            let _ = self.set_block_state_id(bed_pos, block_state_id);
        }

        let position = self
            .find_bed_stand_up_position(entity_type_id, bed_pos, facing, state.y_rot)
            .unwrap_or([
                f64::from(bed_pos.x) + 0.5,
                f64::from(bed_pos.y) + 1.1,
                f64::from(bed_pos.z) + 0.5,
            ]);
        let dx = f64::from(bed_pos.x) + 0.5 - position[0];
        let dz = f64::from(bed_pos.z) + 0.5 - position[2];
        let y_rot = wrap_degrees_f32(dz.atan2(dx).to_degrees() as f32 - 90.0);
        Some(LivingDeathWakeTransition {
            position: EntityVec3 {
                x: position[0],
                y: position[1],
                z: position[2],
            },
            y_rot,
        })
    }

    fn find_bed_stand_up_position(
        &self,
        entity_type_id: i32,
        bed_pos: BlockPos,
        forward: BedModelFacing,
        y_rot: f32,
    ) -> Option<[f64; 3]> {
        let right = forward.clockwise();
        let (right_x, right_z) = right.step();
        let radians = y_rot.to_radians();
        let view_x = -radians.sin();
        let view_z = radians.cos();
        let side = if right_x as f32 * view_x + right_z as f32 * view_z > 0.0 {
            right.opposite()
        } else {
            right
        };
        let surround = bed_surround_stand_up_offsets(forward, side);
        let above = bed_above_stand_up_offsets(forward);
        let below = BlockPos {
            y: bed_pos.y.saturating_sub(1),
            ..bed_pos
        };
        let bunk = self
            .probe_block(below)
            .and_then(|block| block.block_name)
            .is_some_and(|name| block_name_is_bed(&name));

        if bunk {
            self.find_stand_up_at_offsets(entity_type_id, bed_pos, &surround, true)
                .or_else(|| self.find_stand_up_at_offsets(entity_type_id, below, &surround, true))
                .or_else(|| self.find_stand_up_at_offsets(entity_type_id, bed_pos, &above, true))
                .or_else(|| {
                    self.find_stand_up_at_offsets(entity_type_id, bed_pos, &surround, false)
                })
                .or_else(|| self.find_stand_up_at_offsets(entity_type_id, below, &surround, false))
                .or_else(|| self.find_stand_up_at_offsets(entity_type_id, bed_pos, &above, false))
        } else {
            let offsets = surround
                .into_iter()
                .chain(above)
                .collect::<Vec<(i32, i32)>>();
            self.find_stand_up_at_offsets(entity_type_id, bed_pos, &offsets, true)
                .or_else(|| self.find_stand_up_at_offsets(entity_type_id, bed_pos, &offsets, false))
        }
    }

    fn find_stand_up_at_offsets(
        &self,
        entity_type_id: i32,
        origin: BlockPos,
        offsets: &[(i32, i32)],
        check_dangerous: bool,
    ) -> Option<[f64; 3]> {
        for (offset_x, offset_z) in offsets {
            let pos = BlockPos {
                x: origin.x.saturating_add(*offset_x),
                y: origin.y,
                z: origin.z.saturating_add(*offset_z),
            };
            if check_dangerous
                && self
                    .probe_block(pos)
                    .is_some_and(|block| block_is_dangerous(entity_type_id, &block))
            {
                continue;
            }
            let Some(location) = entity_type_safe_dismount_geometry(self, entity_type_id, pos)
            else {
                continue;
            };
            if check_dangerous
                && location[1] - f64::from(pos.y) <= 0.0
                && self
                    .probe_block(BlockPos {
                        y: pos.y.saturating_sub(1),
                        ..pos
                    })
                    .is_some_and(|block| block_is_dangerous(entity_type_id, &block))
            {
                continue;
            }
            return Some(location);
        }
        None
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        if self
            .entities
            .apply_client_animation_hurt(packet.id)
            .is_none()
        {
            self.counters.entity_hurt_animations_ignored += 1;
            return false;
        }
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.id, |events| events.last_hurt_yaw = Some(packet.yaw))
        else {
            self.counters.entity_hurt_animations_ignored += 1;
            return false;
        };
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        if !self
            .entities
            .entity_type_id(packet.source_id)
            .is_some_and(vanilla_entity_metadata_type_is_leashable)
        {
            self.counters.entity_link_updates_ignored += 1;
            return false;
        }
        let Some(()) = self.entities.with_leash_mut(packet.source_id, |leash| {
            leash.holder_id = if packet.dest_id == 0 {
                None
            } else {
                Some(packet.dest_id)
            };
        }) else {
            self.counters.entity_link_updates_ignored += 1;
            return false;
        };
        self.counters.entity_link_updates_applied += 1;
        true
    }

    pub fn apply_set_entity_motion(&mut self, packet: ProtocolSetEntityMotion) -> bool {
        self.counters.entity_motion_updates_received += 1;
        let Some(()) = self.entities.with_transform_mut(packet.id, |transform| {
            transform.delta_movement = entity_vec3(packet.delta_movement);
        }) else {
            self.counters.entity_motion_updates_ignored += 1;
            return false;
        };
        self.counters.entity_motion_updates_applied += 1;
        true
    }

    pub fn apply_rotate_head(&mut self, packet: ProtocolRotateHead) -> bool {
        self.counters.entity_head_rotations_received += 1;
        let Some(()) = self.entities.lerp_head_to(packet.id, packet.y_head_rot, 3) else {
            self.counters.entity_head_rotations_ignored += 1;
            return false;
        };
        self.counters.entity_head_rotations_applied += 1;
        true
    }
}

fn bed_surround_stand_up_offsets(
    forward: BedModelFacing,
    side: BedModelFacing,
) -> [(i32, i32); 10] {
    let (forward_x, forward_z) = forward.step();
    let (side_x, side_z) = side.step();
    [
        (side_x, side_z),
        (side_x - forward_x, side_z - forward_z),
        (side_x - forward_x * 2, side_z - forward_z * 2),
        (-forward_x * 2, -forward_z * 2),
        (-side_x - forward_x * 2, -side_z - forward_z * 2),
        (-side_x - forward_x, -side_z - forward_z),
        (-side_x, -side_z),
        (-side_x + forward_x, -side_z + forward_z),
        (forward_x, forward_z),
        (side_x + forward_x, side_z + forward_z),
    ]
}

fn bed_above_stand_up_offsets(forward: BedModelFacing) -> [(i32, i32); 2] {
    let (forward_x, forward_z) = forward.step();
    [(0, 0), (-forward_x, -forward_z)]
}

fn block_is_dangerous(entity_type_id: i32, block: &BlockProbe) -> bool {
    let Some(name) = block.block_name.as_deref() else {
        return false;
    };
    let burning = matches!(
        name,
        "minecraft:fire"
            | "minecraft:soul_fire"
            | "minecraft:lava"
            | "minecraft:magma_block"
            | "minecraft:lava_cauldron"
    ) || (block_name_is_campfire(name)
        && block.block_properties.get("lit").map(String::as_str) == Some("true"));
    if burning && !vanilla_entity_type_is_fire_immune(entity_type_id) {
        return true;
    }
    match name {
        "minecraft:wither_rose" => !matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_WITHER_ID | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID
        ),
        "minecraft:sweet_berry_bush" => entity_type_id != VANILLA_ENTITY_TYPE_FOX_ID,
        "minecraft:cactus" => true,
        "minecraft:powder_snow" => !matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_POLAR_BEAR_ID
                | VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID
                | VANILLA_ENTITY_TYPE_STRAY_ID
        ),
        _ => false,
    }
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let wrapped = degrees.rem_euclid(360.0);
    if wrapped >= 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}
