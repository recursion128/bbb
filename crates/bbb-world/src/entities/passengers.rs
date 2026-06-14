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
        if !self.entities.contains(packet.vehicle_id) {
            return false;
        }

        self.entities.for_each_mut(|entity| {
            if entity.vehicle_id == Some(packet.vehicle_id) {
                entity.vehicle_id = None;
            }
        });
        self.entities
            .with_mut(packet.vehicle_id, |vehicle| vehicle.passengers.clear());

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
            let Some(old_vehicle_id) = self
                .entities
                .get(passenger_id)
                .and_then(|entity| entity.vehicle_id)
            else {
                let known_passenger = self
                    .entities
                    .with_mut(passenger_id, |passenger| {
                        passenger.vehicle_id = Some(packet.vehicle_id);
                    })
                    .is_some();
                if known_passenger || is_local_player {
                    mounted.push(passenger_id);
                }
                continue;
            };
            if old_vehicle_id != packet.vehicle_id {
                self.remove_passenger_from_vehicle(old_vehicle_id, passenger_id);
            }
            self.entities.with_mut(passenger_id, |passenger| {
                passenger.vehicle_id = Some(packet.vehicle_id);
            });
            mounted.push(passenger_id);
        }

        if local_player_was_on_packet_vehicle && !local_player_mounted_here {
            self.local_player_vehicle_id = None;
        }
        self.entities.with_mut(packet.vehicle_id, |vehicle| {
            vehicle.passengers = mounted;
        });
        self.counters.entity_passenger_updates_applied += 1;
        true
    }

    pub fn apply_move_vehicle(&mut self, packet: ProtocolMoveVehicle) -> Option<VehicleMoveReport> {
        self.counters.vehicle_moves_received += 1;
        let root_vehicle_id = self.local_player_root_vehicle_id()?;
        let packet_position = entity_vec3(packet.position);
        let snapped = entity_distance_squared(
            self.entities.transform(root_vehicle_id)?.position,
            packet_position,
        ) > MOVE_VEHICLE_SNAP_EPSILON_SQUARED;

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

        self.counters.vehicle_moves_applied += 1;
        self.counters.vehicle_moves_acked += 1;
        let transform = self.entities.transform(root_vehicle_id)?;
        Some(VehicleMoveReport {
            vehicle_id: root_vehicle_id,
            position: transform.position,
            y_rot: transform.y_rot,
            x_rot: transform.x_rot,
            on_ground: transform.on_ground.unwrap_or(false),
            snapped,
        })
    }

    pub fn local_player_root_vehicle_id(&self) -> Option<i32> {
        self.resolve_root_vehicle_id(self.local_player_vehicle_id?)
    }

    pub(crate) fn clear_local_player_mount(&mut self, local_player_id: i32) {
        self.local_player_vehicle_id = None;
        self.entities.for_each_mut(|entity| {
            if entity.id == local_player_id {
                entity.vehicle_id = None;
            }
            entity
                .passengers
                .retain(|passenger_id| *passenger_id != local_player_id);
        });
    }

    fn remove_passenger_from_vehicle(&mut self, vehicle_id: i32, passenger_id: i32) {
        self.entities.with_mut(vehicle_id, |vehicle| {
            vehicle
                .passengers
                .retain(|existing| *existing != passenger_id);
        });
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
