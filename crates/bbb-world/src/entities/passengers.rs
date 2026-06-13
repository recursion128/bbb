use bbb_protocol::packets::{
    MoveVehicle as ProtocolMoveVehicle, SetPassengers as ProtocolSetPassengers,
};

use crate::WorldStore;

use super::{
    movement::{entity_distance_squared, entity_vec3},
    VehicleMoveReport,
};

const MOVE_VEHICLE_SNAP_EPSILON_SQUARED: f64 = 1e-10;

impl WorldStore {
    pub fn apply_set_passengers(&mut self, packet: ProtocolSetPassengers) -> bool {
        self.counters.entity_passenger_updates_received += 1;
        self.counters.entity_passenger_ids_received += packet.passenger_ids.len();
        let local_player_id = self.local_player_id;
        let local_player_was_on_packet_vehicle =
            self.local_player_vehicle_id == Some(packet.vehicle_id);
        let Some(vehicle_index) = self
            .entities
            .iter()
            .position(|entity| entity.id == packet.vehicle_id)
        else {
            return false;
        };

        for entity in &mut self.entities {
            if entity.vehicle_id == Some(packet.vehicle_id) {
                entity.vehicle_id = None;
            }
        }
        self.entities[vehicle_index].passengers.clear();

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
            let passenger_index = self
                .entities
                .iter()
                .position(|entity| entity.id == passenger_id);
            let Some(passenger_index) = passenger_index else {
                if is_local_player {
                    mounted.push(passenger_id);
                }
                continue;
            };
            if let Some(old_vehicle_id) = self.entities[passenger_index].vehicle_id {
                if let Some(old_vehicle) = self
                    .entities
                    .iter_mut()
                    .find(|entity| entity.id == old_vehicle_id)
                {
                    old_vehicle
                        .passengers
                        .retain(|existing| *existing != passenger_id);
                }
            }
            self.entities[passenger_index].vehicle_id = Some(packet.vehicle_id);
            mounted.push(passenger_id);
        }

        if local_player_was_on_packet_vehicle && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        self.entities[vehicle_index].passengers = mounted;
        self.counters.entity_passenger_updates_applied += 1;
        true
    }

    pub fn apply_move_vehicle(&mut self, packet: ProtocolMoveVehicle) -> Option<VehicleMoveReport> {
        self.counters.vehicle_moves_received += 1;
        let root_vehicle_id = self.local_player_root_vehicle_id()?;
        let root_vehicle_index = self
            .entities
            .iter()
            .position(|entity| entity.id == root_vehicle_id)?;
        let packet_position = entity_vec3(packet.position);
        let snapped =
            entity_distance_squared(self.entities[root_vehicle_index].position, packet_position)
                > MOVE_VEHICLE_SNAP_EPSILON_SQUARED;

        if snapped {
            let vehicle = &mut self.entities[root_vehicle_index];
            vehicle.position = packet_position;
            vehicle.position_base = packet_position;
            vehicle.y_rot = packet.y_rot;
            vehicle.x_rot = packet.x_rot;
            self.counters.vehicle_moves_snapped += 1;
        }

        self.counters.vehicle_moves_applied += 1;
        self.counters.vehicle_moves_acked += 1;
        let vehicle = &self.entities[root_vehicle_index];
        Some(VehicleMoveReport {
            vehicle_id: vehicle.id,
            position: vehicle.position,
            y_rot: vehicle.y_rot,
            x_rot: vehicle.x_rot,
            on_ground: vehicle.on_ground.unwrap_or(false),
            snapped,
        })
    }

    pub fn local_player_root_vehicle_id(&self) -> Option<i32> {
        self.resolve_root_vehicle_id(self.local_player_vehicle_id?)
    }

    pub(crate) fn clear_local_player_mount(&mut self, local_player_id: i32) {
        self.local_player_vehicle_id = None;
        for entity in &mut self.entities {
            if entity.id == local_player_id {
                entity.vehicle_id = None;
            }
            entity
                .passengers
                .retain(|passenger_id| *passenger_id != local_player_id);
        }
    }

    fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        if let Some(vehicle) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == vehicle_id)
        {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        }
    }

    fn resolve_root_vehicle_id(&self, vehicle_id: i32) -> Option<i32> {
        let mut root_vehicle_id = vehicle_id;
        for _ in 0..self.entities.len() {
            let vehicle = self.probe_entity(root_vehicle_id)?;
            let Some(parent_vehicle_id) = vehicle.vehicle_id else {
                return Some(root_vehicle_id);
            };
            root_vehicle_id = parent_vehicle_id;
        }
        None
    }
}
