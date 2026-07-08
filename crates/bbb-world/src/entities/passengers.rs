use bbb_protocol::packets::{
    MoveVehicle as ProtocolMoveVehicle, SetPassengers as ProtocolSetPassengers,
};

use crate::{LocalPlayerInputState, WorldStore};

use super::{
    is_vanilla_boat_type,
    movement::{entity_distance_squared, entity_vec3},
    VehicleMoveReport, VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID, VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_HORSE_ID,
    VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_MULE_ID,
    VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
    VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};

const MOVE_VEHICLE_SNAP_EPSILON_SQUARED: f64 = 1e-10;
const LOCAL_BOAT_TICK_SECONDS: f64 = 0.05;
const LOCAL_BOAT_TURN_DEGREES_PER_TICK: f32 = 1.0;
const LOCAL_BOAT_FORWARD_ACCELERATION_PER_TICK: f64 = 0.04;
const LOCAL_BOAT_BACKWARD_ACCELERATION_PER_TICK: f64 = -0.005;
const LOCAL_BOAT_TURN_ONLY_ACCELERATION_PER_TICK: f64 = 0.005;
const LOCAL_BOAT_HORIZONTAL_DAMPING: f64 = 0.9;

impl WorldStore {
    pub fn apply_set_passengers(&mut self, packet: ProtocolSetPassengers) -> bool {
        self.counters.entity_passenger_updates_received += 1;
        self.counters.entity_passenger_ids_received += packet.passenger_ids.len();
        let local_player_id = self.local_player_id;
        let local_player_was_on_packet_vehicle =
            self.local_player_vehicle_id == Some(packet.vehicle_id);
        if !self.entities.contains(packet.vehicle_id) {
            self.counters.entity_passenger_updates_ignored += 1;
            return false;
        }

        self.entities.for_each_mount_mut(|_, mount| {
            if mount.vehicle_id == Some(packet.vehicle_id) {
                mount.vehicle_id = None;
            }
        });
        self.entities
            .with_mount_mut(packet.vehicle_id, |vehicle| vehicle.passengers.clear());

        let mut mounted = Vec::new();
        let mut local_player_mounted_here = false;
        for passenger_id in packet.passenger_ids {
            if passenger_id == packet.vehicle_id || mounted.contains(&passenger_id) {
                continue;
            }
            let is_local_player = local_player_id == Some(passenger_id);
            if is_local_player {
                if let Some(old_vehicle_id) = self.local_player_vehicle_id {
                    if old_vehicle_id != packet.vehicle_id {
                        self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
                    }
                }
                self.local_player_vehicle_id = Some(packet.vehicle_id);
                local_player_mounted_here = true;
            }
            match self.entities.mount(passenger_id) {
                Some(passenger_mount) => {
                    if let Some(old_vehicle_id) = passenger_mount.vehicle_id {
                        if old_vehicle_id != packet.vehicle_id {
                            self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
                        }
                    }
                    self.entities.with_mount_mut(passenger_id, |passenger| {
                        passenger.vehicle_id = Some(packet.vehicle_id);
                    });
                    mounted.push(passenger_id);
                }
                None => {
                    if is_local_player {
                        mounted.push(passenger_id);
                    }
                }
            }
        }

        if local_player_was_on_packet_vehicle && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        self.entities.with_mount_mut(packet.vehicle_id, |vehicle| {
            vehicle.passengers = mounted;
        });
        self.counters.entity_passenger_updates_applied += 1;
        true
    }

    pub fn apply_move_vehicle(&mut self, packet: ProtocolMoveVehicle) -> Option<VehicleMoveReport> {
        self.counters.vehicle_moves_received += 1;
        let Some(root_vehicle_id) = self.local_player_root_vehicle_id() else {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        };
        let packet_position = entity_vec3(packet.position);
        let Some(current_transform) = self.entities.transform(root_vehicle_id) else {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        };
        let snapped = entity_distance_squared(current_transform.position, packet_position)
            > MOVE_VEHICLE_SNAP_EPSILON_SQUARED;

        if snapped {
            self.entities
                .with_transform_mut(root_vehicle_id, |transform| {
                    transform.position = packet_position;
                    transform.position_base = packet_position;
                    transform.y_rot = packet.y_rot;
                    transform.x_rot = packet.x_rot;
                });
            self.counters.vehicle_moves_snapped += 1;
        }

        let Some(transform) = self.entities.transform(root_vehicle_id) else {
            self.counters.vehicle_moves_ignored += 1;
            return None;
        };
        self.counters.vehicle_moves_applied += 1;
        self.counters.vehicle_moves_acked += 1;
        Some(VehicleMoveReport {
            vehicle_id: root_vehicle_id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            on_ground: transform.on_ground.unwrap_or(false),
            snapped,
        })
    }

    pub fn advance_local_boat_vehicle_input(
        &mut self,
        input: LocalPlayerInputState,
        dt_seconds: f64,
    ) -> Option<VehicleMoveReport> {
        let vehicle_id = self.local_player_root_boat_vehicle_id()?;
        let ticks = (dt_seconds.max(0.0) / LOCAL_BOAT_TICK_SECONDS).max(0.0);
        self.entities.with_transform_mut(vehicle_id, |transform| {
            if ticks > 0.0 {
                let turn = if input.focused {
                    axis(input.right, input.left)
                } else {
                    0.0
                };
                transform.y_rot = wrap_degrees_f32(
                    transform.y_rot + turn as f32 * LOCAL_BOAT_TURN_DEGREES_PER_TICK * ticks as f32,
                );

                let mut acceleration = 0.0;
                if input.focused {
                    if input.forward {
                        acceleration += LOCAL_BOAT_FORWARD_ACCELERATION_PER_TICK;
                    }
                    if input.backward {
                        acceleration += LOCAL_BOAT_BACKWARD_ACCELERATION_PER_TICK;
                    }
                    if turn != 0.0 && !input.forward && !input.backward {
                        acceleration += LOCAL_BOAT_TURN_ONLY_ACCELERATION_PER_TICK;
                    }
                }
                acceleration *= ticks;

                let yaw = f64::from(transform.y_rot).to_radians();
                transform.delta_movement.x += (-yaw.sin()) * acceleration;
                transform.delta_movement.z += yaw.cos() * acceleration;
                let damping = LOCAL_BOAT_HORIZONTAL_DAMPING.powf(ticks);
                transform.delta_movement.x *= damping;
                transform.delta_movement.z *= damping;
                transform.position.x += transform.delta_movement.x * ticks;
                transform.position.y += transform.delta_movement.y * ticks;
                transform.position.z += transform.delta_movement.z * ticks;
                transform.position_base = transform.position;
            }
        })?;

        let transform = self.entities.transform(vehicle_id)?;
        Some(VehicleMoveReport {
            vehicle_id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            on_ground: transform.on_ground.unwrap_or(false),
            snapped: false,
        })
    }

    pub fn local_player_root_vehicle_id(&self) -> Option<i32> {
        self.resolve_root_vehicle_id(self.local_player_vehicle_id?)
    }

    pub fn entity_body_anchor_y_offset(
        &self,
        entity_id: i32,
        is_front: bool,
        partial_ticks: f32,
    ) -> Option<f32> {
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        self.entities
            .body_anchor_y_offset(entity_id, game_time, is_front, partial_ticks)
    }

    pub fn local_player_root_boat_vehicle_id(&self) -> Option<i32> {
        let vehicle_id = self.local_player_root_vehicle_id()?;
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_boat_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub fn local_player_rideable_jumping_vehicle_id(&self) -> Option<i32> {
        let (Some(local_player_id), Some(vehicle_id)) =
            (self.local_player_id, self.local_player_vehicle_id)
        else {
            return None;
        };
        let mount = self.entities.mount(vehicle_id)?;
        if mount.passengers.first().copied() != Some(local_player_id) {
            return None;
        }
        let entity_type_id = self.entities.entity_type_id(vehicle_id)?;
        if !is_vanilla_player_rideable_jumping_type(entity_type_id) {
            return None;
        }
        if !self
            .entities
            .saddle_slot_contains_saddle_item(vehicle_id, &self.items.default_item_equipment_slots)
        {
            return None;
        }
        Some(vehicle_id)
    }

    /// Vanilla `PlayerRideableJumping.getJumpCooldown()` for the local
    /// player's controlled jumpable mount. `local_player_rideable_jumping_vehicle_id`
    /// applies the shared `canJump()` saddle gate. Non-cooldown mount classes
    /// use the interface default of zero; currently tracked cooldown classes
    /// are camel and camel husk. Nautilus dash cooldown is not yet
    /// reconstructed.
    pub fn local_player_rideable_jumping_vehicle_cooldown(
        &self,
        partial_ticks: f32,
    ) -> Option<f32> {
        let vehicle_id = self.local_player_rideable_jumping_vehicle_id()?;
        self.entities
            .player_rideable_jumping_cooldown(vehicle_id, partial_ticks)
            .map(|cooldown| cooldown.max(0.0))
    }

    pub fn local_player_sprintable_vehicle_id(&self) -> Option<i32> {
        let (Some(local_player_id), Some(vehicle_id)) =
            (self.local_player_id, self.local_player_vehicle_id)
        else {
            return None;
        };
        let mount = self.entities.mount(vehicle_id)?;
        if mount.passengers.first().copied() != Some(local_player_id) {
            return None;
        }
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_sprintable_vehicle_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub fn local_player_server_controlled_inventory_vehicle_id(&self) -> Option<i32> {
        let vehicle_id = self.local_player_vehicle_id?;
        self.entities
            .entity_type_id(vehicle_id)
            .filter(|entity_type_id| is_vanilla_custom_inventory_screen_type(*entity_type_id))
            .map(|_| vehicle_id)
    }

    pub(crate) fn clear_local_player_mount(&mut self, local_player_id: i32) {
        self.local_player_vehicle_id = None;
        self.entities.for_each_mount_mut(|entity_id, mount| {
            if entity_id == local_player_id {
                mount.vehicle_id = None;
            }
            mount
                .passengers
                .retain(|passenger_id| *passenger_id != local_player_id);
        });
    }

    fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        self.entities.with_mount_mut(vehicle_id, |vehicle| {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        });
    }

    fn resolve_root_vehicle_id(&self, vehicle_id: i32) -> Option<i32> {
        let mut root_vehicle_id = vehicle_id;
        for _ in 0..self.entities.len() {
            let mount = self.entities.mount(root_vehicle_id)?;
            let Some(parent_vehicle_id) = mount.vehicle_id else {
                return Some(root_vehicle_id);
            };
            root_vehicle_id = parent_vehicle_id;
        }
        None
    }
}

fn axis(positive: bool, negative: bool) -> f64 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn wrap_degrees_f32(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

fn is_vanilla_player_rideable_jumping_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID
            | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_LLAMA_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    )
}

fn is_vanilla_sprintable_vehicle_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
    )
}

fn is_vanilla_custom_inventory_screen_type(entity_type_id: i32) -> bool {
    is_vanilla_player_rideable_jumping_type(entity_type_id)
        || matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_ACACIA_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
                | VANILLA_ENTITY_TYPE_BIRCH_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_CHERRY_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_DARK_OAK_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_JUNGLE_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_MANGROVE_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_PALE_OAK_CHEST_BOAT_ID
                | VANILLA_ENTITY_TYPE_SPRUCE_CHEST_BOAT_ID
        )
}
