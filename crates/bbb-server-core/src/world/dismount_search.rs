//! Where a living rider lands when it leaves its vehicle: the vehicle's
//! `getDismountLocationForPassenger` and the free-position fallback.

use bbb_facts::client_information_options::ClientMainHand;
use bbb_protocol::entity_poses::{
    VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_STANDING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{mth_cos, mth_sin, static_block_collision_box_iter, Aabb};
use bbb_server_types::interaction::{player_pose_dimensions, WorldBorder};

use super::movement_collision::block_collision_boxes;
use super::nether_portal::{closest_free_center, shape_within_border};
use super::simulation_level::LevelCtx;
use super::stored_block_state;
use crate::chunk::ChunkStore;
use crate::entity::general_living::is_abstract_horse_entity_type;
use crate::entity::Entity;

/// [`DismountLevel::living_dismount_target`] for a mob leaving a vehicle
/// inside its own tick.
pub(crate) fn detached_mob_dismount_target(
    level: &LevelCtx<'_, '_>,
    vehicle: &Entity,
    rider: &Entity,
) -> (Vec3d, i32) {
    DismountLevel {
        chunks: &*level.chunks,
        border: level.world_border,
        game_time: level.game_time,
    }
    .living_dismount_target_inner(
        Some(vehicle),
        vehicle.position(),
        DismountRider::Stored(rider),
        level.minecart_improvements_enabled(),
    )
}

/// The level reads a dismount search makes.
pub(in crate::world) struct DismountLevel<'a> {
    pub(in crate::world) chunks: &'a ChunkStore,
    pub(in crate::world) border: WorldBorder,
    pub(in crate::world) game_time: i64,
}

#[derive(Clone, Copy)]
pub(in crate::world) enum DismountRider<'a> {
    Player(&'a super::PlayerState),
    Stored(&'a crate::entity::Entity),
}

impl DismountRider<'_> {
    fn position(self) -> Vec3d {
        match self {
            Self::Player(p) => p.position(),
            Self::Stored(e) => e.position(),
        }
    }
    fn pose(self) -> i32 {
        match self {
            Self::Player(p) => p.pose_id,
            Self::Stored(e) => e.pose_id(),
        }
    }
    fn poses(self) -> &'static [i32] {
        match self {
            Self::Player(_) => &DISMOUNT_POSES,
            Self::Stored(_) => &[VANILLA_ENTITY_POSE_STANDING_ID],
        }
    }
    fn dimensions(self, pose: i32) -> [f32; 2] {
        match self {
            Self::Player(p) => {
                player_pose_dimensions(pose, p.pose_scale()).expect("player pose exists")
            }
            Self::Stored(e) => {
                if pose == e.pose_id() {
                    return [e.bb_width(), e.bb_height()];
                }
                let geometry = match e.data() {
                    crate::entity::EntityData::ArmorStand(data) => {
                        Some(crate::entity::armor_stand::geometry(data))
                    }
                    _ => crate::entity::general_living::dimensions::geometry(
                        e.data(),
                        e.entity_type_id(),
                        pose,
                    ),
                };
                geometry
                    .map(|[w, h, _]| [w, h])
                    .unwrap_or([e.bb_width(), e.bb_height()])
            }
        }
    }
    fn y_rot(self) -> f32 {
        match self {
            Self::Player(p) => p.rotation().y_rot,
            Self::Stored(e) => e.y_rot(),
        }
    }
    fn main_arm(self) -> ClientMainHand {
        match self {
            Self::Player(p) => p.cold.client_information.main_hand,
            Self::Stored(e) => {
                if matches!(e.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_left_handed())
                {
                    ClientMainHand::Left
                } else {
                    ClientMainHand::Right
                }
            }
        }
    }
}

impl DismountLevel<'_> {
    /// `LivingEntity.dismountVehicle` (`LivingEntity.java:2301-2323`) for a
    /// rider still in the level, with the vehicle at `vehicle_position`.
    pub(in crate::world) fn living_dismount_target(
        &self,
        vehicle: Option<&Entity>,
        vehicle_position: Vec3d,
        rider: DismountRider<'_>,
    ) -> (Vec3d, i32) {
        self.living_dismount_target_inner(vehicle, vehicle_position, rider, false)
    }

    fn living_dismount_target_inner(
        &self,
        vehicle: Option<&Entity>,
        vehicle_position: Vec3d,
        rider: DismountRider<'_>,
        minecart_improvements: bool,
    ) -> (Vec3d, i32) {
        let position = rider.position();
        let in_portal = self.block_in_portals_tag(BlockPos {
            x: vehicle_position.x.floor() as i32,
            y: vehicle_position.y.floor() as i32,
            z: vehicle_position.z.floor() as i32,
        });
        let Some(vehicle) =
            vehicle.filter(|vehicle| !in_portal && vehicle.removal_reason().is_none())
        else {
            // `:2305-2319`: the vehicle is gone or standing in a portal, so the
            // rider rises to the higher of the two and is nudged out of any
            // block it would be inside.
            let max_y = position.y.max(vehicle_position.y);
            let target = Vec3d {
                y: max_y,
                ..position
            };
            return (
                self.free_dismount_center(target, rider.pose(), rider),
                rider.pose(),
            );
        };
        let current = vehicle.position();
        let vehicle_bounds = vehicle.bounding_box().moved(Vec3d {
            x: vehicle_position.x - current.x,
            y: vehicle_position.y - current.y,
            z: vehicle_position.z - current.z,
        });
        let vehicle_width = vehicle.bb_width();
        let vehicle_y_rot = vehicle.y_rot();
        let vehicle_type_id = vehicle.entity_type_id();
        let is_boat = crate::entity::boat::is_boat(vehicle.data());
        let [player_width, _] = rider.dimensions(VANILLA_ENTITY_POSE_STANDING_ID);
        if is_boat {
            return self.boat_dismount_location(
                (vehicle_position, vehicle_bounds, vehicle_width),
                rider.y_rot(),
                player_width,
                rider,
                rider.pose(),
            );
        }
        if is_abstract_horse_entity_type(vehicle_type_id) {
            // `AbstractHorse.getDismountLocationForPassenger`
            // (`AbstractHorse.java:998-1012`): the main-hand side first, then
            // the off-hand side, then the horse's own position.
            let main_arm = rider.main_arm();
            for side in [main_arm, opposite_arm(main_arm)] {
                let offset = if side == ClientMainHand::Right {
                    90.0
                } else {
                    -90.0
                };
                let direction = collision_horizontal_escape_vector(
                    f64::from(vehicle_width),
                    f64::from(player_width),
                    vehicle_y_rot + offset,
                );
                if let Some(location) = self.dismount_location_in_direction(
                    vehicle_position,
                    vehicle_bounds,
                    direction,
                    rider,
                ) {
                    return location;
                }
            }
            return (vehicle_position, rider.pose());
        }
        let vehicle_block = BlockPos {
            x: vehicle_position.x.floor() as i32,
            y: vehicle_position.y.floor() as i32,
            z: vehicle_position.z.floor() as i32,
        };
        let found = if crate::entity::minecart::is_minecart(vehicle.data()) {
            let facing = facing_step(vehicle.y_rot());
            let forward = if minecart_improvements || vehicle.has_minecart_track_interpolation() {
                // `MinecartBehavior.getMotionDirection` (`MinecartBehavior.java:90-92`),
                // inherited by `NewMinecartBehavior`.
                facing
            } else if crate::entity::minecart::is_flipped(vehicle) {
                // `OldMinecartBehavior.getMotionDirection` (`OldMinecartBehavior.java:395-397`).
                clockwise((-facing.0, -facing.1))
            } else {
                clockwise(facing)
            };
            self.minecart_dismount_location(vehicle_block, forward, rider)
        } else if vehicle_type_id == VANILLA_ENTITY_TYPE_STRIDER_ID {
            self.strider_dismount_location(vehicle_position, vehicle_bounds, vehicle_width, rider)
        } else if vehicle_type_id != VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            && matches!(vehicle.data(), crate::entity::EntityData::GeneralLiving(data)
                if data.animal_retained().is_some())
        {
            self.animal_dismount_location(vehicle_block, facing_step(vehicle.y_rot()), rider)
        } else {
            None
        };
        found.unwrap_or((
            crate::entity::passenger::default_dismount_position(
                vehicle_position,
                vehicle_bounds.max_y,
            ),
            rider.pose(),
        ))
    }

    /// `AbstractMinecart.getDismountLocationForPassenger` (`AbstractMinecart.java:184-237`).
    fn minecart_dismount_location(
        &self,
        vehicle_block: BlockPos,
        forward: (i32, i32),
        rider: DismountRider<'_>,
    ) -> Option<(Vec3d, i32)> {
        let offsets = dismount_offsets(forward);
        for &pose_id in rider.poses() {
            let [width, height] = rider.dimensions(pose_id);
            let reach = f64::from(width.min(1.0) / 2.0);
            // `AbstractMinecart.POSE_DISMOUNT_HEIGHTS` (`:55-57`).
            let heights: &[i32] = if pose_id == VANILLA_ENTITY_POSE_SWIMMING_ID {
                &[0, 1]
            } else {
                &[0, 1, -1]
            };
            for &offset_y in heights {
                for (dx, dz) in offsets {
                    let target = BlockPos {
                        x: vehicle_block.x + dx,
                        y: vehicle_block.y + offset_y,
                        z: vehicle_block.z + dz,
                    };
                    let floor = self.block_floor_height_with(target, true);
                    if !is_block_floor_valid(floor) {
                        continue;
                    }
                    let location = up_from_bottom_center(target, floor);
                    let bounds = Aabb {
                        min_x: location.x - reach,
                        min_y: location.y,
                        min_z: location.z - reach,
                        max_x: location.x + reach,
                        max_y: location.y + f64::from(height),
                        max_z: location.z + reach,
                    };
                    if self.can_dismount_into(bounds) {
                        return Some((location, pose_id));
                    }
                }
            }
        }
        None
    }

    /// `Animal.getDismountLocationForPassenger` (`Animal.java:249-276`).
    fn animal_dismount_location(
        &self,
        vehicle_block: BlockPos,
        forward: (i32, i32),
        rider: DismountRider<'_>,
    ) -> Option<(Vec3d, i32)> {
        let offsets = dismount_offsets(forward);
        for &pose_id in rider.poses() {
            for (dx, dz) in offsets {
                let target = BlockPos {
                    x: vehicle_block.x + dx,
                    z: vehicle_block.z + dz,
                    ..vehicle_block
                };
                let floor = self.block_floor_height(target);
                if !is_block_floor_valid(floor) {
                    continue;
                }
                let location = up_from_bottom_center(target, floor);
                if self.can_dismount_to(location, pose_id, rider) {
                    return Some((location, pose_id));
                }
            }
        }
        None
    }

    /// `Strider.getDismountLocationForPassenger` (`Strider.java:203-243`): five
    /// escape directions around the rider's facing, never onto lava.
    fn strider_dismount_location(
        &self,
        vehicle_position: Vec3d,
        vehicle_bounds: Aabb,
        vehicle_width: f32,
        rider: DismountRider<'_>,
    ) -> Option<(Vec3d, i32)> {
        let [rider_width, _] = rider.dimensions(rider.pose());
        let y_rot = rider.y_rot();
        let top = vehicle_bounds.max_y;
        let bottom = vehicle_bounds.min_y - 0.5;
        let mut targets = Vec::new();
        for offset in [0.0, -22.5, 22.5, -45.0, 45.0] {
            let direction = collision_horizontal_escape_vector(
                f64::from(vehicle_width),
                f64::from(rider_width),
                y_rot + offset,
            );
            let mut pos = BlockPos {
                x: (vehicle_position.x + direction.x).floor() as i32,
                y: top.floor() as i32,
                z: (vehicle_position.z + direction.z).floor() as i32,
            };
            let mut y = top;
            while y > bottom {
                if !targets.contains(&pos) {
                    targets.push(pos);
                }
                pos.y -= 1;
                y -= 1.0;
            }
        }
        for target in targets {
            let state = stored_block_state(self.chunks, target);
            if block_state_fluid_state(state).family() == Some(FluidFamily::Lava) {
                continue;
            }
            let floor = self.block_floor_height(target);
            if !is_block_floor_valid(floor) {
                continue;
            }
            let location = up_from_bottom_center(target, floor);
            for &pose_id in rider.poses() {
                if self.can_dismount_to(location, pose_id, rider) {
                    return Some((location, pose_id));
                }
            }
        }
        None
    }

    /// `AbstractHorse.getDismountLocationInDirection` (`AbstractHorse.java:965-1004`).
    fn dismount_location_in_direction(
        &self,
        vehicle_position: Vec3d,
        vehicle_bounds: Aabb,
        direction: Vec3d,
        rider: DismountRider<'_>,
    ) -> Option<(Vec3d, i32)> {
        let target_x = vehicle_position.x + direction.x;
        let target_z = vehicle_position.z + direction.z;
        let jump_limit = vehicle_bounds.max_y + 0.75;
        for &pose_id in rider.poses() {
            let mut block_y = vehicle_bounds.min_y.floor() as i32;
            loop {
                let pos = BlockPos {
                    x: target_x.floor() as i32,
                    y: block_y,
                    z: target_z.floor() as i32,
                };
                let floor_height = self.block_floor_height(pos);
                if f64::from(block_y) + floor_height > jump_limit {
                    break;
                }
                if is_block_floor_valid(floor_height) {
                    let location = Vec3d {
                        x: target_x,
                        y: f64::from(block_y) + floor_height,
                        z: target_z,
                    };
                    if self.can_dismount_to(location, pose_id, rider) {
                        return Some((location, pose_id));
                    }
                }
                block_y += 1;
                if f64::from(block_y) >= jump_limit {
                    break;
                }
            }
        }
        None
    }

    /// `AbstractBoat.getDismountLocationForPassenger` (`AbstractBoat.java:630-658`).
    fn boat_dismount_location(
        &self,
        (vehicle_position, vehicle_bounds, vehicle_width): (Vec3d, Aabb, f32),
        player_y_rot: f32,
        player_width: f32,
        rider: DismountRider<'_>,
        previous_pose: i32,
    ) -> (Vec3d, i32) {
        // `Mth.SQRT_OF_TWO` (`Mth.java:26`) multiplies the width in float
        // before the argument widens.
        let direction = collision_horizontal_escape_vector(
            f64::from(vehicle_width * 2.0_f32.sqrt()),
            f64::from(player_width),
            player_y_rot,
        );
        let target_x = vehicle_position.x + direction.x;
        let target_z = vehicle_position.z + direction.z;
        let target = BlockPos {
            x: target_x.floor() as i32,
            y: vehicle_bounds.max_y.floor() as i32,
            z: target_z.floor() as i32,
        };
        let below = BlockPos {
            y: target.y - 1,
            ..target
        };
        if !self.is_water_at(below) {
            let mut candidates = Vec::new();
            let target_floor = self.block_floor_height(target);
            if is_block_floor_valid(target_floor) {
                candidates.push(Vec3d {
                    x: target_x,
                    y: f64::from(target.y) + target_floor,
                    z: target_z,
                });
            }
            let below_floor = self.block_floor_height(below);
            if is_block_floor_valid(below_floor) {
                candidates.push(Vec3d {
                    x: target_x,
                    y: f64::from(below.y) + below_floor,
                    z: target_z,
                });
            }
            for &pose_id in rider.poses() {
                for &candidate in &candidates {
                    if self.can_dismount_to(candidate, pose_id, rider) {
                        return (candidate, pose_id);
                    }
                }
            }
        }
        (
            Vec3d {
                x: vehicle_position.x,
                y: vehicle_bounds.max_y,
                z: vehicle_position.z,
            },
            previous_pose,
        )
    }

    /// `DismountHelper.canDismountTo` (`DismountHelper.java:41-49`).
    fn can_dismount_to(&self, location: Vec3d, pose_id: i32, rider: DismountRider<'_>) -> bool {
        let [width, height] = rider.dimensions(pose_id);
        self.can_dismount_into(Aabb::of_entity(location, width, height))
    }

    fn can_dismount_into(&self, bounds: Aabb) -> bool {
        block_collision_boxes(self.chunks, bounds).is_empty()
            && shape_within_border(bounds, self.border, self.game_time)
    }

    /// `BlockGetter.getBlockFloorHeight` (`BlockGetter.java:96-110`);
    /// unknown collision shapes fail closed as full blocks.
    fn block_floor_height(&self, pos: BlockPos) -> f64 {
        self.block_floor_height_with(pos, false)
    }

    /// `non_climbable` reads `DismountHelper.nonClimbableShape`
    /// (`DismountHelper.java:55-60`): climbables and open trapdoors are empty.
    fn block_floor_height_with(&self, pos: BlockPos, non_climbable: bool) -> f64 {
        let chunks = self.chunks;
        let tags = chunks.registry_tags_shared();
        // `None` is `VoxelShape.isEmpty()`; `Some` is its `max(Axis.Y)`.
        let shape_max_y = |pos: BlockPos| {
            let state = stored_block_state(chunks, pos);
            if non_climbable {
                let block =
                    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state);
                let tagged = |tag: &str| {
                    block.is_some_and(|block| {
                        tags.as_ref()
                            .is_some_and(|tags| tags.contains_block(tag, block))
                    })
                };
                if tagged("minecraft:climbable")
                    || tagged("minecraft:trapdoors")
                        && bbb_protocol::block_state_types::vanilla_block_state_property(
                            state, "open",
                        ) == Some("true")
                {
                    return None;
                }
            }
            let Some(boxes) = static_block_collision_box_iter(state) else {
                return Some(1.0);
            };
            boxes.map(|shape| shape.max_y).reduce(f64::max)
        };
        if let Some(here) = shape_max_y(pos) {
            return here;
        }
        match shape_max_y(BlockPos {
            y: pos.y - 1,
            ..pos
        }) {
            Some(below) if below >= 1.0 => below - 1.0,
            _ => f64::NEG_INFINITY,
        }
    }

    /// `BlockState.is(BlockTags.PORTALS)` at the vehicle's block position.
    fn block_in_portals_tag(&self, pos: BlockPos) -> bool {
        let state = stored_block_state(self.chunks, pos);
        let Some(block_id) =
            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        else {
            return false;
        };
        self.chunks
            .registry_tags_shared()
            .is_some_and(|tags| tags.contains_block("minecraft:portals", block_id))
    }

    /// `LevelReader.isWaterAt` (`LevelReader.java:136-138`).
    fn is_water_at(&self, pos: BlockPos) -> bool {
        let state = stored_block_state(self.chunks, pos);
        block_state_fluid_state(state).family() == Some(FluidFamily::Water)
    }

    /// `LivingEntity.dismountVehicle`'s `findFreePosition` arm
    /// (`LivingEntity.java:2308-2318`).
    fn free_dismount_center(&self, target: Vec3d, pose_id: i32, rider: DismountRider<'_>) -> Vec3d {
        let [width, height] = rider.dimensions(pose_id);
        if width > 4.0 || height > 4.0 {
            return target;
        }
        let half_height = f64::from(height) / 2.0;
        let center = Vec3d {
            y: target.y + half_height,
            ..target
        };
        let allowed = Aabb::of_size(
            center,
            f64::from(width),
            f64::from(height),
            f64::from(width),
        );
        let search = allowed.inflate(f64::from(width), f64::from(height), f64::from(width));
        let obstacles = block_collision_boxes(self.chunks, search)
            .into_iter()
            .filter(|shape| shape_within_border(*shape, self.border, self.game_time))
            .map(|shape| shape.inflate(f64::from(width) / 2.0, half_height, f64::from(width) / 2.0))
            .collect::<Vec<_>>();
        closest_free_center(allowed, center, &obstacles).map_or(target, |free| Vec3d {
            y: free.y - half_height,
            ..free
        })
    }
}

const fn opposite_arm(arm: ClientMainHand) -> ClientMainHand {
    match arm {
        ClientMainHand::Left => ClientMainHand::Right,
        ClientMainHand::Right => ClientMainHand::Left,
    }
}

/// `Entity.getCollisionHorizontalEscapeVector` (`Entity.java:3524-3530`).
fn collision_horizontal_escape_vector(
    collider_width: f64,
    colliding_width: f64,
    direction_degrees: f32,
) -> Vec3d {
    let distance = (collider_width + colliding_width + f64::from(1.0e-5_f32)) / 2.0;
    let radians = direction_degrees * (std::f64::consts::PI / 180.0) as f32;
    let direction_x = -mth_sin(f64::from(radians));
    let direction_z = mth_cos(f64::from(radians));
    let scale = direction_x.abs().max(direction_z.abs());
    Vec3d {
        x: f64::from(direction_x) * distance / f64::from(scale),
        y: 0.0,
        z: f64::from(direction_z) * distance / f64::from(scale),
    }
}

/// `Direction.fromYRot(yRot)` as a horizontal step.
fn facing_step(y_rot: f32) -> (i32, i32) {
    match (f64::from(y_rot) / 90.0 + 0.5).floor() as i32 & 3 {
        0 => (0, 1),
        1 => (-1, 0),
        2 => (0, -1),
        _ => (1, 0),
    }
}

/// `Direction.getClockWise` on a horizontal step.
const fn clockwise((x, z): (i32, i32)) -> (i32, i32) {
    (-z, x)
}

/// `DismountHelper.offsetsForDirection` (`DismountHelper.java:21-35`).
fn dismount_offsets(forward: (i32, i32)) -> [(i32, i32); 8] {
    let (fx, fz) = forward;
    let (rx, rz) = clockwise(forward);
    let (lx, lz) = (-rx, -rz);
    let (bx, bz) = (-fx, -fz);
    [
        (rx, rz),
        (lx, lz),
        (bx + rx, bz + rz),
        (bx + lx, bz + lz),
        (fx + rx, fz + rz),
        (fx + lx, fz + lz),
        (bx, bz),
        (fx, fz),
    ]
}

/// `Vec3.upFromBottomCenterOf(pos, height)`.
fn up_from_bottom_center(pos: BlockPos, height: f64) -> Vec3d {
    Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + height,
        z: f64::from(pos.z) + 0.5,
    }
}

/// `DismountHelper.isBlockFloorValid` (`DismountHelper.java:37-39`).
fn is_block_floor_valid(block_floor_height: f64) -> bool {
    !block_floor_height.is_infinite() && block_floor_height < 1.0
}

/// `Player.getDismountPoses` (`Player.java:1849-1851`).
const DISMOUNT_POSES: [i32; 3] = [
    VANILLA_ENTITY_POSE_STANDING_ID,
    VANILLA_ENTITY_POSE_CROUCHING_ID,
    VANILLA_ENTITY_POSE_SWIMMING_ID,
];
