use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROAKING_ID, VANILLA_ENTITY_POSE_CROUCHING_ID,
    VANILLA_ENTITY_POSE_DYING_ID, VANILLA_ENTITY_POSE_INHALING_ID,
    VANILLA_ENTITY_POSE_LONG_JUMPING_ID, VANILLA_ENTITY_POSE_ROARING_ID,
    VANILLA_ENTITY_POSE_SHOOTING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
    VANILLA_ENTITY_POSE_SLIDING_ID, VANILLA_ENTITY_POSE_SNIFFING_ID,
    VANILLA_ENTITY_POSE_STANDING_ID, VANILLA_ENTITY_POSE_USING_TONGUE_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_passenger_attachments, vanilla_entity_type_vehicle_attachment,
    VANILLA_ENTITY_TYPE_BREEZE_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID,
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID, VANILLA_ENTITY_TYPE_DONKEY_ID,
    VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    VANILLA_ENTITY_TYPE_MULE_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
    VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
    VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_WARDEN_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::packets::Vec3d;

use crate::entity::environment::FluidTag;
use crate::entity::general_living::is_base_animal_entity_type;
use crate::entity::{Entity, EntityData};

#[derive(Clone, Copy)]
enum VehicleGeometry {
    Generated,
    Horse,
    Camel,
    Llama,
    Slime,
    Strider,
    Boat {
        ride_height_factor: f32,
        single_passenger_offset: f32,
        max_passengers: usize,
    },
    Minecart,
}

#[derive(Clone, Copy)]
struct BoatRotation {
    vehicle_yaw: f32,
    delta_rotation: f32,
    full_animal_offset: Option<f32>,
}

pub(super) struct PassengerPlacement {
    position: Vec3d,
    boat_rotation: Option<BoatRotation>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalPlayerVehiclePlacement {
    pub position: Vec3d,
    pub y_rot: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum RelationAdmission {
    Normal,
    Forced,
}

impl PassengerPlacement {
    pub(super) fn apply(self, passenger: &mut Entity) {
        passenger.set_pos(self.position);
        self.apply_rotation(passenger);
    }

    pub(super) fn apply_snap(self, passenger: &mut Entity) -> bool {
        passenger.set_pos(self.position);
        passenger.set_old_pos_and_rot();
        let rotates = self.boat_rotation.is_some();
        self.apply_rotation(passenger);
        rotates
    }

    fn apply_rotation(self, passenger: &mut Entity) {
        let Some(rotation) = self.boat_rotation else {
            return;
        };

        let y_rot = passenger.y_rot() + rotation.delta_rotation;
        let delta = crate::entity::living::wrap_degrees(y_rot - rotation.vehicle_yaw);
        let y_rot = y_rot + delta.clamp(-105.0, 105.0) - delta;
        passenger.set_rot(y_rot, passenger.x_rot());
        if let Some(living) = passenger.living_mut() {
            living.y_body_rot = rotation.vehicle_yaw;
            living.y_head_rot = y_rot;
            if let Some(offset) = rotation.full_animal_offset {
                living.y_body_rot += offset;
                living.y_head_rot += offset;
            }
        }
    }
}

pub(crate) fn relation_supported(
    admission: RelationAdmission,
    vehicle: &Entity,
    passenger: &Entity,
    passenger_index: usize,
    passenger_count: usize,
    game_time: i64,
) -> bool {
    geometry(vehicle).is_some_and(|geometry| {
        vehicle_could_accept(vehicle)
            && bbb_protocol::entity_types::vanilla_entity_type_is_serializable(
                vehicle.entity_type_id(),
            )
            && geometry.representable(passenger_index, passenger_count)
            && (admission == RelationAdmission::Forced
                || geometry.accepts_normal(vehicle, passenger_count))
            && geometry
                .attachment(
                    vehicle,
                    passenger.entity_type_id(),
                    is_base_animal_entity_type(passenger.entity_type_id()),
                    passenger_index,
                    passenger_count,
                    game_time,
                )
                .is_some()
            && vanilla_entity_type_vehicle_attachment(passenger.entity_type_id()).is_some()
    })
}

pub(super) fn placement(
    vehicle: &Entity,
    passenger: &Entity,
    passenger_index: usize,
    passenger_count: usize,
    game_time: i64,
) -> Option<PassengerPlacement> {
    let geometry = geometry(vehicle)?;
    if !geometry.representable(passenger_index, passenger_count) {
        return None;
    }
    // `Entity.java:2319-2333` combines the vehicle's passenger attachment
    // with the rider's vehicle attachment.
    let vehicle_offset = geometry.attachment(
        vehicle,
        passenger.entity_type_id(),
        is_base_animal_entity_type(passenger.entity_type_id()),
        passenger_index,
        passenger_count,
        game_time,
    )?;
    let rider_offset = match spider_vehicle_attachment(vehicle, passenger) {
        Some(offset) => offset,
        None => rotate_y(
            crate::entity::general_living::dimensions::vehicle_attachment(passenger)?,
            passenger.y_rot(),
        ),
    };
    let boat_rotation = matches!(geometry, VehicleGeometry::Boat { .. })
        .then(|| boat_rotation(vehicle, passenger, passenger_count))
        .flatten();
    Some(PassengerPlacement {
        position: Vec3d {
            x: vehicle.position().x + vehicle_offset.x - rider_offset.x,
            y: vehicle.position().y + vehicle_offset.y - rider_offset.y,
            z: vehicle.position().z + vehicle_offset.z - rider_offset.z,
        },
        boat_rotation,
    })
}

/// `Spider.getVehicleAttachmentPoint` (`Spider.java:177-178`) and
/// `CaveSpider.java:59-60`, on a vehicle no wider than the spider.
fn spider_vehicle_attachment(vehicle: &Entity, passenger: &Entity) -> Option<Vec3d> {
    let height = match passenger.entity_type_id() {
        VANILLA_ENTITY_TYPE_SPIDER_ID => 0.3125,
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => 0.21875,
        _ => return None,
    };
    if vehicle.bb_width() > passenger.bb_width() {
        return None;
    }
    let scale = passenger
        .living()?
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID);
    Some(Vec3d {
        x: 0.0,
        y: height * scale,
        z: 0.0,
    })
}

pub(crate) fn external_player_relation_supported(
    admission: RelationAdmission,
    vehicle: &Entity,
    passenger_index: usize,
    passenger_count: usize,
    game_time: i64,
) -> bool {
    geometry(vehicle).is_some_and(|geometry| {
        vehicle_could_accept(vehicle)
            && geometry.representable(passenger_index, passenger_count)
            && (admission == RelationAdmission::Forced
                || geometry.accepts_normal(vehicle, passenger_count))
            && geometry
                .attachment(
                    vehicle,
                    VANILLA_ENTITY_TYPE_PLAYER_ID,
                    false,
                    passenger_index,
                    passenger_count,
                    game_time,
                )
                .is_some()
            && vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_PLAYER_ID).is_some()
    })
}

pub(crate) fn external_player_relation_restorable(
    vehicle: &Entity,
    passenger_index: usize,
    passenger_count: usize,
) -> bool {
    vehicle_could_accept(vehicle)
        && passenger_index < passenger_count
        && vanilla_entity_type_vehicle_attachment(VANILLA_ENTITY_TYPE_PLAYER_ID).is_some()
}

pub(crate) fn external_player_placement(
    vehicle: &Entity,
    passenger_index: usize,
    passenger_count: usize,
    player_yaw: f32,
    player_scale: f32,
    player_pose_id: i32,
    game_time: i64,
) -> Option<ExternalPlayerVehiclePlacement> {
    if !player_yaw.is_finite() {
        return None;
    }
    let geometry = geometry(vehicle)?;
    if !geometry.representable(passenger_index, passenger_count) {
        return None;
    }
    // `Entity.java:2319-2333` combines the vehicle attachment with the
    // player's scaled, pose-specific, rotated vehicle attachment.
    let vehicle_offset = geometry.attachment(
        vehicle,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
        false,
        passenger_index,
        passenger_count,
        game_time,
    )?;
    let rider_offset = rotate_y(
        crate::entity::general_living::dimensions::avatar_vehicle_attachment(
            player_pose_id,
            player_scale,
        ),
        player_yaw,
    );
    let y_rot = match geometry {
        VehicleGeometry::Boat { .. } => rotated_boat_yaw(vehicle, player_yaw)?,
        _ => player_yaw,
    };
    Some(ExternalPlayerVehiclePlacement {
        position: Vec3d {
            x: vehicle.position().x + vehicle_offset.x - rider_offset.x,
            y: vehicle.position().y + vehicle_offset.y - rider_offset.y,
            z: vehicle.position().z + vehicle_offset.z - rider_offset.z,
        },
        y_rot,
    })
}

pub(crate) fn external_player_controls_boat(vehicle: &Entity, passenger_index: usize) -> bool {
    // `AbstractBoat.java:744-745` returns its first living passenger.
    passenger_index == 0 && matches!(geometry(vehicle), Some(VehicleGeometry::Boat { .. }))
}

impl VehicleGeometry {
    fn representable(self, passenger_index: usize, passenger_count: usize) -> bool {
        passenger_index < passenger_count
    }

    fn accepts_normal(self, vehicle: &Entity, passenger_count: usize) -> bool {
        match self {
            // `AbstractBoat.java:735-740`; `AbstractChestBoat.java:43-49`.
            Self::Boat { max_passengers, .. } => {
                !vehicle.is_eye_in_fluid(FluidTag::Water) && passenger_count <= max_passengers
            }
            Self::Minecart | Self::Horse | Self::Llama | Self::Slime => passenger_count <= 1,
            // `Camel.java:560-561` checks the pre-add size against two.
            Self::Camel => passenger_count <= 3,
            Self::Generated => match vehicle.entity_type_id() {
                // `HappyGhast.java:329-330`.
                VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID => passenger_count <= 4,
                // `AbstractNautilus.java:165-166`.
                VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
                    passenger_count <= 1
                }
                // `Entity.java:2452-2454`.
                _ => passenger_count <= 1,
            },
            // `Strider.java:355-356`.
            Self::Strider => !vehicle.is_eye_in_fluid(FluidTag::Lava) && passenger_count <= 1,
        }
    }

    fn attachment(
        self,
        vehicle: &Entity,
        passenger_type_id: i32,
        passenger_is_animal: bool,
        passenger_index: usize,
        passenger_count: usize,
        game_time: i64,
    ) -> Option<Vec3d> {
        let mut point = self.base_attachment(
            vehicle,
            passenger_type_id,
            passenger_is_animal,
            passenger_index,
            passenger_count,
            game_time,
        )?;
        if matches!(self, Self::Horse) {
            let EntityData::GeneralLiving(data) = vehicle.data() else {
                return None;
            };
            // AbstractHorse.java:1038-1041; LivingEntity.java:3916-3917.
            let amount = data.abstract_horse_retained()?.rearing_amount();
            let scale = living_scale(vehicle)? * living_age_scale(vehicle)?;
            let offset = rotate_y(
                [0.0, 0.15 * amount * scale, -0.7 * amount * scale],
                vehicle.y_rot(),
            );
            point.x += offset.x;
            point.y += offset.y;
            point.z += offset.z;
        }
        Some(point)
    }

    fn base_attachment(
        self,
        vehicle: &Entity,
        passenger_type_id: i32,
        passenger_is_animal: bool,
        passenger_index: usize,
        passenger_count: usize,
        game_time: i64,
    ) -> Option<Vec3d> {
        // LivingEntity.java:3663-3669: sleeping uses the fixed box's default attachments.
        if matches!(self, Self::Generated | Self::Horse)
            && vehicle.living().is_some()
            && vehicle.pose_id() == VANILLA_ENTITY_POSE_SLEEPING_ID
        {
            return Some(rotate_y([0.0, vehicle.bb_height(), 0.0], vehicle.y_rot()));
        }
        // Goat.java:55,245-246; Warden.java:512-514: fresh boxes use default attachments.
        if matches!(self, Self::Generated)
            && ((vehicle.entity_type_id() == VANILLA_ENTITY_TYPE_GOAT_ID
                && vehicle.pose_id() == VANILLA_ENTITY_POSE_LONG_JUMPING_ID)
                || crate::entity::warden::is_digging_or_emerging(
                    vehicle.entity_type_id(),
                    vehicle.pose_id(),
                ))
        {
            return Some(rotate_y([0.0, vehicle.bb_height(), 0.0], vehicle.y_rot()));
        }
        if let Some(point) =
            crate::entity::general_living::dimensions::baby_passenger_attachment(vehicle)
        {
            return Some(rotate_y(point, vehicle.y_rot()));
        }
        match self {
            Self::Generated => {
                let scale = generated_living_geometry_scale(vehicle)?;
                // Sniffer.java:73-75,122-124: digging replaces the type's attachments with defaults.
                let digging_sniffer = matches!(vehicle.data(), EntityData::GeneralLiving(data)
                    if data.sniffer_state().is_some_and(|state| state.is_digging()));
                let attachment = if digging_sniffer {
                    [0.0, vehicle.bb_height(), 0.0]
                } else {
                    vanilla_entity_type_passenger_attachments(vehicle.entity_type_id())
                        .map(|attachments| {
                            attachments[passenger_index.min(attachments.len() - 1)]
                                .map(|coordinate| coordinate * scale)
                        })
                        .unwrap_or([0.0, vehicle.bb_height(), 0.0])
                };
                Some(rotate_y(attachment, vehicle.y_rot()))
            }
            Self::Horse => {
                if !matches!(
                    vehicle.pose_id(),
                    VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID
                ) {
                    return None;
                }
                let scale = living_scale(vehicle)?;
                let attachment =
                    vanilla_entity_type_passenger_attachments(vehicle.entity_type_id())
                        .map(|attachments| attachments[passenger_index.min(attachments.len() - 1)])
                        .unwrap_or([0.0, vehicle_type_height(vehicle)?, 0.0]);
                Some(rotate_y(
                    [
                        attachment[0] * scale,
                        attachment[1] * scale,
                        attachment[2] * scale,
                    ],
                    vehicle.y_rot(),
                ))
            }
            Self::Camel => {
                // `Camel.java:489-505` uses the live index and passenger
                // count, not the generated base attachment.
                let driver = passenger_index == 0;
                let mut z_offset = if driver { 0.5 } else { -0.7 };
                if passenger_count > 1 && passenger_is_animal {
                    z_offset += 0.2;
                }
                let (height, scale) = crate::entity::camel::passenger_attachment_height_and_scale(
                    vehicle, driver, game_time,
                )?;
                Some(rotate_y([0.0, height, z_offset * scale], vehicle.y_rot()))
            }
            Self::Llama => {
                // `Llama.java:444-450`.
                let living_scale = living_scale(vehicle)?;
                if vehicle.pose_id() == VANILLA_ENTITY_POSE_SLEEPING_ID {
                    return Some(rotate_y([0.0, 0.2, 0.0], vehicle.y_rot()));
                }
                if !matches!(
                    vehicle.pose_id(),
                    VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID
                ) {
                    return None;
                }
                let attachments =
                    vanilla_entity_type_passenger_attachments(vehicle.entity_type_id())?;
                let attachment = attachments[passenger_index.min(attachments.len() - 1)];
                Some(rotate_y(
                    [
                        attachment[0] * living_scale,
                        attachment[1] * living_scale,
                        attachment[2] * living_scale,
                    ],
                    vehicle.y_rot(),
                ))
            }
            Self::Slime => {
                // `Slime.java:245-247`, `Slime.java:346-348`.
                let EntityData::GeneralLiving(data) = vehicle.data() else {
                    return None;
                };
                let size = data.slime_retained()?.size() as f32;
                let scale = living_scale(vehicle)?;
                let dimensions_height = if vehicle.pose_id() == VANILLA_ENTITY_POSE_SLEEPING_ID {
                    0.2_f32
                } else if matches!(
                    vehicle.pose_id(),
                    VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID
                ) {
                    vehicle_type_height(vehicle)? * size * scale
                } else {
                    return None;
                };
                Some(Vec3d {
                    x: 0.0,
                    y: f64::from(dimensions_height)
                        - 0.015_625_f64 * f64::from(size) * f64::from(scale),
                    z: 0.0,
                })
            }
            Self::Strider => {
                // `Strider.java:179-187` delegates to the current scaled
                // dimensions on the server.
                let height = if vehicle.pose_id() == VANILLA_ENTITY_POSE_SLEEPING_ID {
                    0.2
                } else if matches!(
                    vehicle.pose_id(),
                    VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID
                ) {
                    vehicle_type_height(vehicle)?
                        * living_scale(vehicle)?
                        * living_age_scale(vehicle)?
                } else {
                    return None;
                };
                Some(Vec3d {
                    x: 0.0,
                    y: f64::from(height),
                    z: 0.0,
                })
            }
            Self::Minecart
                if matches!(
                    passenger_type_id,
                    VANILLA_ENTITY_TYPE_VILLAGER_ID | VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
                ) =>
            {
                // `AbstractMinecart.java:178-180`.
                Some(Vec3d {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                })
            }
            Self::Minecart => {
                let attachments =
                    vanilla_entity_type_passenger_attachments(vehicle.entity_type_id())?;
                Some(rotate_y(
                    attachments[passenger_index.min(attachments.len() - 1)],
                    vehicle.y_rot(),
                ))
            }
            Self::Boat {
                ride_height_factor,
                single_passenger_offset,
                ..
            } => {
                // `AbstractBoat.java:132-150`, `Boat.java:15-17`, and
                // `Raft.java:15-17`.
                let mut offset = single_passenger_offset;
                if passenger_count > 1 {
                    offset = if passenger_index == 0 { 0.2 } else { -0.6 };
                    if passenger_is_animal {
                        offset += 0.2;
                    }
                }
                Some(rotate_y(
                    [0.0, vehicle.bb_height() * ride_height_factor, offset],
                    vehicle.y_rot(),
                ))
            }
        }
    }
}

fn geometry(vehicle: &Entity) -> Option<VehicleGeometry> {
    let entity_type_id = vehicle.entity_type_id();
    if matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HORSE_ID
            | VANILLA_ENTITY_TYPE_DONKEY_ID
            | VANILLA_ENTITY_TYPE_MULE_ID
            | VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID
            | VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
    ) {
        return Some(VehicleGeometry::Horse);
    }
    let geometry = match vehicle.data() {
        EntityData::Boat(_) | EntityData::ChestBoat(_) => VehicleGeometry::Boat {
            ride_height_factor: if matches!(
                entity_type_id,
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BAMBOO_RAFT_ID
                    | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BAMBOO_CHEST_RAFT_ID
            ) {
                0.888_888_9
            } else {
                1.0 / 3.0
            },
            single_passenger_offset: if matches!(vehicle.data(), EntityData::ChestBoat(_)) {
                0.15
            } else {
                0.0
            },
            max_passengers: if matches!(vehicle.data(), EntityData::ChestBoat(_)) {
                1
            } else {
                2
            },
        },
        EntityData::Minecart(_)
        | EntityData::FurnaceMinecart(_)
        | EntityData::ChestMinecart(_)
        | EntityData::HopperMinecart(_)
        | EntityData::CommandBlockMinecart(_)
        | EntityData::SpawnerMinecart(_)
        | EntityData::TntMinecart(_) => VehicleGeometry::Minecart,
        _ if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_CAMEL_ID | VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        ) =>
        {
            VehicleGeometry::Camel
        }
        _ if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
        ) =>
        {
            VehicleGeometry::Llama
        }
        _ if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_SLIME_ID | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
        ) =>
        {
            VehicleGeometry::Slime
        }
        _ if entity_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID => VehicleGeometry::Strider,
        _ => VehicleGeometry::Generated,
    };
    Some(geometry)
}

fn living_scale(entity: &Entity) -> Option<f32> {
    let raw = entity
        .living()?
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
        as f32;
    Some(crate::entity::general_living::dimensions::sanitize_scale(
        entity.entity_type_id(),
        raw,
    ))
}

fn vehicle_type_height(entity: &Entity) -> Option<f32> {
    if let EntityData::GeneralLiving(data) = entity.data() {
        if let Some(plugin) = data.plugin_mob() {
            return Some(plugin.height);
        }
    }
    bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity.entity_type_id())
        .map(|dimensions| dimensions[1])
}

fn living_age_scale(entity: &Entity) -> Option<f32> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return None;
    };
    Some(
        if data
            .ageable_retained()
            .is_some_and(|ageable| ageable.is_baby())
        {
            0.5
        } else {
            1.0
        },
    )
}

// LivingEntity.java:3663-3669; EntityDimensions.java:29-39: age and scale also scale attachments.
fn generated_living_geometry_scale(entity: &Entity) -> Option<f32> {
    let EntityData::GeneralLiving(_) = entity.data() else {
        return Some(1.0);
    };
    if !matches!(
        (entity.entity_type_id(), entity.pose_id()),
        (
            _,
            VANILLA_ENTITY_POSE_STANDING_ID | VANILLA_ENTITY_POSE_DYING_ID
        ) | (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            VANILLA_ENTITY_POSE_CROUCHING_ID
                | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID
                | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_FALL_FLYING_ID
                | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SPIN_ATTACK_ID
                | bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
        ) | (
            VANILLA_ENTITY_TYPE_CAT_ID | VANILLA_ENTITY_TYPE_OCELOT_ID,
            VANILLA_ENTITY_POSE_CROUCHING_ID
        ) | (
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            VANILLA_ENTITY_POSE_ROARING_ID | VANILLA_ENTITY_POSE_SNIFFING_ID
        ) | (
            VANILLA_ENTITY_TYPE_BREEZE_ID,
            VANILLA_ENTITY_POSE_SLIDING_ID
                | VANILLA_ENTITY_POSE_SHOOTING_ID
                | VANILLA_ENTITY_POSE_INHALING_ID
                | VANILLA_ENTITY_POSE_LONG_JUMPING_ID
        ) | (
            VANILLA_ENTITY_TYPE_FROG_ID,
            VANILLA_ENTITY_POSE_CROAKING_ID
                | VANILLA_ENTITY_POSE_LONG_JUMPING_ID
                | VANILLA_ENTITY_POSE_USING_TONGUE_ID
        )
    ) {
        return None;
    }
    crate::entity::general_living::dimensions::generated_passenger_scale(entity)
}

fn vehicle_could_accept(vehicle: &Entity) -> bool {
    // `Marker.java:42-48`; `OminousItemSpawner.java:129-135`.
    !matches!(
        vehicle.data(),
        EntityData::Marker(_) | EntityData::OminousItemSpawner(_)
    )
}

fn boat_rotation(
    vehicle: &Entity,
    passenger: &Entity,
    passenger_count: usize,
) -> Option<BoatRotation> {
    // `AbstractBoat.java:615-627`; `AbstractBoat.java:661-668`.
    if passenger.entity_type_id() == VANILLA_ENTITY_TYPE_BREEZE_ID {
        return None;
    }
    let max_passengers = match vehicle.data() {
        EntityData::ChestBoat(_) => 1,
        _ => 2,
    };
    let full_animal_offset = (passenger_count == max_passengers
        && is_base_animal_entity_type(passenger.entity_type_id()))
    .then_some(if passenger.id() % 2 == 0 { 90.0 } else { 270.0 });
    Some(BoatRotation {
        vehicle_yaw: vehicle.y_rot(),
        delta_rotation: crate::entity::boat::boat(vehicle.data())?.delta_rotation(),
        full_animal_offset,
    })
}

fn rotated_boat_yaw(vehicle: &Entity, rider_yaw: f32) -> Option<f32> {
    // `AbstractBoat.java:615-627`; `AbstractBoat.java:661-668`.
    let y_rot = rider_yaw + crate::entity::boat::boat(vehicle.data())?.delta_rotation();
    let delta = crate::entity::living::wrap_degrees(y_rot - vehicle.y_rot());
    Some(y_rot + delta.clamp(-105.0, 105.0) - delta)
}

fn rotate_y(point: [f32; 3], yaw: f32) -> Vec3d {
    let radians = -f64::from(yaw).to_radians();
    let (sin, cos) = radians.sin_cos();
    let x = f64::from(point[0]);
    let z = f64::from(point[2]);
    Vec3d {
        x: x * cos + z * sin,
        y: f64::from(point[1]),
        z: z * cos - x * sin,
    }
}
