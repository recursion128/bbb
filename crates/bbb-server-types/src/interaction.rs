//! How far a player's hand reaches, and what the far end of it does.

use bbb_protocol::{
    attribute_types::vanilla_attribute_type,
    entity_anchors::EntityAnchor,
    entity_poses::{
        VANILLA_ENTITY_POSE_CROUCHING_ID, VANILLA_ENTITY_POSE_DYING_ID,
        VANILLA_ENTITY_POSE_FALL_FLYING_ID, VANILLA_ENTITY_POSE_SLEEPING_ID,
        VANILLA_ENTITY_POSE_SPIN_ATTACK_ID, VANILLA_ENTITY_POSE_SWIMMING_ID,
    },
    entity_types::{
        vanilla_entity_type_dimensions, vanilla_entity_type_eye_height,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    },
    packets::{BlockPos, Vec3d},
};

use crate::geometry::Aabb;
use crate::{Result, ServerWorldError};
use bbb_facts::default_attributes_26_1::vanilla_entity_default_attributes_26_1;

/// The resource id the generated default-attribute table keys the player by.
pub const PLAYER_RESOURCE_ID: &str = "minecraft:player";

/// `EntityType.PLAYER`'s standing size for consumers without session pose.
pub fn player_bounding_box(position: Vec3d) -> Result<Aabb> {
    let [width, height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_PLAYER_ID)
        .ok_or(ServerWorldError::MissingCatalogFact("player dimensions"))?;
    Ok(Aabb::of_entity(position, width, height))
}

/// `Avatar.POSES` and `LivingEntity.getDimensions` (Avatar.java:19-34;
/// LivingEntity.java:3663): the pose dimensions are scaled before boxing.
pub fn player_pose_dimensions(pose_id: i32, scale: f32) -> Result<[f32; 2]> {
    // Fixed dimensions ignore scale (Avatar.java:36; EntityDimensions.java:29-39).
    if matches!(
        pose_id,
        VANILLA_ENTITY_POSE_SLEEPING_ID | VANILLA_ENTITY_POSE_DYING_ID
    ) {
        return Ok([0.2, 0.2]);
    }
    let [width, standing_height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_PLAYER_ID)
        .ok_or(ServerWorldError::MissingCatalogFact("player dimensions"))?;
    let height = match pose_id {
        VANILLA_ENTITY_POSE_CROUCHING_ID => 1.5,
        VANILLA_ENTITY_POSE_SWIMMING_ID
        | VANILLA_ENTITY_POSE_FALL_FLYING_ID
        | VANILLA_ENTITY_POSE_SPIN_ATTACK_ID => 0.6,
        _ => standing_height,
    };
    Ok([width * scale, height * scale])
}

pub fn player_pose_bounding_box(position: Vec3d, pose_id: i32, scale: f32) -> Result<Aabb> {
    let [width, height] = player_pose_dimensions(pose_id, scale)?;
    Ok(Aabb::of_entity(position, width, height))
}

/// `Entity.getEyeHeight()` (Entity.java:3379-3381) for a standing player.
pub fn player_eye_height() -> Result<f32> {
    vanilla_entity_type_eye_height(VANILLA_ENTITY_TYPE_PLAYER_ID)
        .ok_or(ServerWorldError::MissingCatalogFact("player eye height"))
}

/// `Entity.getEyePosition` over `Entity.getEyeY` (Entity.java:3706-3708).
pub fn player_eye_position(position: Vec3d) -> Result<Vec3d> {
    let eye_height = player_eye_height()?;
    Ok(Vec3d {
        x: position.x,
        y: position.y + f64::from(eye_height),
        z: position.z,
    })
}

/// `EntityAnchorArgument.Anchor.apply(Entity)` (EntityAnchorArgument.java:81-83)
/// for a player, which is the only entity kind a command can name here.
///
/// `FEET` is the identity on the position (`:61`) and `EYES` raises it by the
/// eye height (`:62`), which is the one [`player_eye_position`] already reads.
pub fn player_anchor_position(position: Vec3d, anchor: EntityAnchor) -> Result<Vec3d> {
    match anchor {
        EntityAnchor::Feet => Ok(position),
        EntityAnchor::Eyes => player_eye_position(position),
    }
}

/// The base value a player's attribute starts at.
///
/// `AttributeSupplier.Builder.add(Attribute)` takes the attribute's registry
/// default and `add(Attribute, double)` declares its own, which is exactly the
/// distinction the generated table records. No modifier source exists in this
/// world, so the base value is also the current value.
pub fn player_attribute_value(attribute_id: i32) -> Result<f64> {
    let declared = vanilla_entity_default_attributes_26_1(PLAYER_RESOURCE_ID)
        .ok_or(ServerWorldError::MissingCatalogFact(
            "player default attributes",
        ))?
        .iter()
        .find(|(candidate, _)| *candidate == attribute_id)
        .ok_or(ServerWorldError::MissingCatalogFact(
            "player attribute base value",
        ))?
        .1;
    match declared {
        Some(base) => Ok(base),
        None => Ok(f64::from_bits(
            vanilla_attribute_type(attribute_id)
                .ok_or(ServerWorldError::MissingCatalogFact("attribute type"))?
                .default_value_bits,
        )),
    }
}

/// `Player.isWithinBlockInteractionRange` (Player.java:1991-1994) over the
/// session's `BLOCK_INTERACTION_RANGE` value.
#[must_use]
pub fn is_within_block_interaction_range(
    eye: Vec3d,
    pos: BlockPos,
    range: f64,
    buffer: f64,
) -> bool {
    let max_range = range + buffer;
    Aabb::of_block(pos).distance_to_sqr(eye) < max_range * max_range
}

/// `Player.isWithinEntityInteractionRange` (Player.java:1981-1985) over the
/// session's `ENTITY_INTERACTION_RANGE` value.
#[must_use]
pub fn is_within_entity_interaction_range(
    eye: Vec3d,
    target: &Aabb,
    range: f64,
    buffer: f64,
) -> bool {
    let max_range = range + buffer;
    target.distance_to_sqr(eye) < max_range * max_range
}

/// `AttackRange` (`AttackRange.java`), the reach band a weapon declares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttackRange {
    pub min_reach: f32,
    pub max_reach: f32,
    pub min_creative_reach: f32,
    pub max_creative_reach: f32,
    pub hitbox_margin: f32,
    /// Only a non-player attacker scales its reach by it
    /// (`AttackRange.effectiveMinRange`, :86-99).
    pub mob_factor: f32,
}

impl AttackRange {
    /// `AttackRange.defaultFor` (AttackRange.java:55-57): a weapon with no
    /// component of its own reaches exactly as far as the attacker's entity
    /// interaction range, in both directions and with no margin.
    #[must_use]
    pub fn default_for_player(entity_interaction_range: f64) -> Self {
        let interaction_range = entity_interaction_range as f32;
        Self {
            min_reach: 0.0,
            max_reach: interaction_range,
            min_creative_reach: 0.0,
            max_creative_reach: interaction_range,
            hitbox_margin: 0.0,
            mob_factor: 1.0,
        }
    }

    /// `AttackRange.effectiveMinRange` (AttackRange.java:86-92) for a player.
    pub fn effective_min_range(&self, creative: bool) -> f32 {
        if creative {
            self.min_creative_reach
        } else {
            self.min_reach
        }
    }

    /// `AttackRange.effectiveMaxRange` (AttackRange.java:94-100) for a player.
    pub fn effective_max_range(&self, creative: bool) -> f32 {
        if creative {
            self.max_creative_reach
        } else {
            self.max_reach
        }
    }

    /// `AttackRange.isInRange` (AttackRange.java:110-116). The comparison is on
    /// the distance itself rather than its square, because the band has a lower
    /// bound that the buffer can push below zero.
    pub fn is_in_range(&self, eye: Vec3d, creative: bool, target: &Aabb, buffer: f64) -> bool {
        let distance = target.distance_to_sqr(eye).sqrt();
        let min_reach =
            f64::from(self.effective_min_range(creative)) - f64::from(self.hitbox_margin) - buffer;
        let max_reach =
            f64::from(self.effective_max_range(creative)) + f64::from(self.hitbox_margin) + buffer;
        distance >= min_reach && distance <= max_reach
    }
}

/// One level's `WorldBorder` geometry and persisted settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldBorder {
    center_x: f64,
    center_z: f64,
    damage_per_block: f64,
    safe_zone: f64,
    warning_blocks: i32,
    warning_time: i32,
    size: f64,
    lerp_time: i64,
    lerp_target: f64,
    lerp_start_game_time: i64,
    absolute_max_size: f64,
}

impl WorldBorder {
    /// `Settings.DEFAULT` (`WorldBorder.java:460-477`) after
    /// `applyInitialSettings` (`WorldBorder.java:285-298`).
    pub const DEFAULT: Self = Self {
        center_x: 0.0,
        center_z: 0.0,
        damage_per_block: 0.2,
        safe_zone: 5.0,
        warning_blocks: 5,
        warning_time: 300,
        size: 5.999997E7_f32 as f64,
        lerp_time: 0,
        lerp_target: 5.999997E7_f32 as f64,
        lerp_start_game_time: 0,
        absolute_max_size: 29_999_984.0,
    };

    /// `WorldBorder.applyInitialSettings` (`WorldBorder.java:285-298`).
    #[allow(clippy::too_many_arguments)]
    pub const fn from_settings(
        center_x: f64,
        center_z: f64,
        damage_per_block: f64,
        safe_zone: f64,
        warning_blocks: i32,
        warning_time: i32,
        size: f64,
        lerp_time: i64,
        lerp_target: f64,
        game_time: i64,
    ) -> Self {
        let moving = lerp_time > 0 && size != lerp_target;
        Self {
            center_x,
            center_z,
            damage_per_block,
            safe_zone,
            warning_blocks,
            warning_time,
            size,
            lerp_time: if moving { lerp_time } else { 0 },
            lerp_target: if moving { lerp_target } else { size },
            lerp_start_game_time: game_time,
            absolute_max_size: 29_999_984.0,
        }
    }

    pub const fn center_x(&self) -> f64 {
        self.center_x
    }

    pub const fn center_z(&self) -> f64 {
        self.center_z
    }

    pub const fn damage_per_block(&self) -> f64 {
        self.damage_per_block
    }

    pub const fn safe_zone(&self) -> f64 {
        self.safe_zone
    }

    pub const fn warning_blocks(&self) -> i32 {
        self.warning_blocks
    }

    pub const fn warning_time(&self) -> i32 {
        self.warning_time
    }

    pub const fn lerp_target(&self) -> f64 {
        self.lerp_target
    }

    /// `MovingBorderExtent.calculateSize` and `update`
    /// (`WorldBorder.java:397-440`).
    pub fn size_at(&self, game_time: i64) -> f64 {
        if self.lerp_time == 0 {
            return self.size;
        }
        let elapsed = game_time
            .saturating_sub(self.lerp_start_game_time)
            .clamp(0, self.lerp_time);
        let progress = elapsed as f64 / self.lerp_time as f64;
        self.size + (self.lerp_target - self.size) * progress
    }

    pub fn remaining_lerp_time(&self, game_time: i64) -> i64 {
        self.lerp_time
            .saturating_sub(game_time.saturating_sub(self.lerp_start_game_time))
            .max(0)
    }

    /// `WorldBorder.StaticBorderExtent.getMinX` (:353-360) after the clamp.
    pub fn min_x(&self) -> f64 {
        self.min_x_at(self.lerp_start_game_time)
    }

    pub fn max_x(&self) -> f64 {
        self.max_x_at(self.lerp_start_game_time)
    }

    pub fn min_z(&self) -> f64 {
        self.min_z_at(self.lerp_start_game_time)
    }

    pub fn max_z(&self) -> f64 {
        self.max_z_at(self.lerp_start_game_time)
    }

    pub fn min_x_at(&self, game_time: i64) -> f64 {
        (self.center_x - self.size_at(game_time) / 2.0)
            .clamp(-self.absolute_max_size, self.absolute_max_size)
    }

    pub fn max_x_at(&self, game_time: i64) -> f64 {
        (self.center_x + self.size_at(game_time) / 2.0)
            .clamp(-self.absolute_max_size, self.absolute_max_size)
    }

    pub fn min_z_at(&self, game_time: i64) -> f64 {
        (self.center_z - self.size_at(game_time) / 2.0)
            .clamp(-self.absolute_max_size, self.absolute_max_size)
    }

    pub fn max_z_at(&self, game_time: i64) -> f64 {
        (self.center_z + self.size_at(game_time) / 2.0)
            .clamp(-self.absolute_max_size, self.absolute_max_size)
    }

    /// `WorldBorder.getDistanceToBorder` (`WorldBorder.java:102-112`).
    pub fn distance_to_border_at(&self, x: f64, z: f64, game_time: i64) -> f64 {
        let from_north = z - self.min_z_at(game_time);
        let from_south = self.max_z_at(game_time) - z;
        let from_west = x - self.min_x_at(game_time);
        let from_east = self.max_x_at(game_time) - x;
        from_west.min(from_east).min(from_north).min(from_south)
    }

    pub fn no_collision_at(
        &self,
        source: Vec3d,
        bounds: crate::geometry::Aabb,
        game_time: i64,
    ) -> bool {
        bounds.min_x >= bounds.max_x
            || bounds.min_y >= bounds.max_y
            || bounds.min_z >= bounds.max_z
            || !self
                .collision_boxes_at(source, bounds, game_time)
                .iter()
                .any(|shape| shape.intersects(bounds))
    }

    pub fn collision_boxes_at(
        &self,
        source: Vec3d,
        bounds: crate::geometry::Aabb,
        game_time: i64,
    ) -> Vec<crate::geometry::Aabb> {
        let min_x = self.min_x_at(game_time);
        let max_x = self.max_x_at(game_time);
        let min_z = self.min_z_at(game_time);
        let max_z = self.max_z_at(game_time);
        // WorldBorder.java:114-116: the actor gate uses the unrounded live extent.
        let margin = (bounds.max_x - bounds.min_x)
            .abs()
            .max((bounds.max_z - bounds.min_z).abs())
            .max(1.0);
        if !(self.distance_to_border_at(source.x, source.z, game_time) < margin * 2.0
            && source.x >= min_x - margin
            && source.x < max_x + margin
            && source.z >= min_z - margin
            && source.z < max_z + margin)
        {
            return Vec::new();
        }
        // WorldBorder.java:444-457: the complement of the rounded interior.
        let infinity = f64::INFINITY;
        vec![
            Aabb {
                min_x: -infinity,
                min_y: -infinity,
                min_z: -infinity,
                max_x: min_x.floor(),
                max_y: infinity,
                max_z: infinity,
            },
            Aabb {
                min_x: max_x.ceil(),
                min_y: -infinity,
                min_z: -infinity,
                max_x: infinity,
                max_y: infinity,
                max_z: infinity,
            },
            Aabb {
                min_x: -infinity,
                min_y: -infinity,
                min_z: -infinity,
                max_x: infinity,
                max_y: infinity,
                max_z: min_z.floor(),
            },
            Aabb {
                min_x: -infinity,
                min_y: -infinity,
                min_z: max_z.ceil(),
                max_x: infinity,
                max_y: infinity,
                max_z: infinity,
            },
        ]
    }

    /// `WorldBorder.isWithinBounds(BlockPos)` (:48-50, :68-74) against the
    /// extent the border actually has at `game_time`, which is what a live
    /// `WorldBorder.getMinX` reads (`WorldBorder.java:123-129`, `:353-358`); the
    /// argumentless [`Self::contains_block`] answers as of the lerp's start
    /// instead and so cannot see a resize in progress.
    pub fn contains_block_at(&self, pos: BlockPos, game_time: i64) -> bool {
        let x = f64::from(pos.x);
        let z = f64::from(pos.z);
        x >= self.min_x_at(game_time)
            && x < self.max_x_at(game_time)
            && z >= self.min_z_at(game_time)
            && z < self.max_z_at(game_time)
    }

    /// `WorldBorder.isWithinBounds(BlockPos)` (:48-50, :68-74). The upper bound
    /// is exclusive on both axes.
    pub fn contains_block(&self, pos: BlockPos) -> bool {
        let x = f64::from(pos.x);
        let z = f64::from(pos.z);
        x >= self.min_x() && x < self.max_x() && z >= self.min_z() && z < self.max_z()
    }
}

#[cfg(test)]
mod border_collision_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::attribute_types::{
        VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID,
        VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID,
    };

    #[test]
    fn a_player_box_is_centred_on_the_position_and_grows_upwards() {
        let position = Vec3d {
            x: 0.5,
            y: 65.0,
            z: -3.5,
        };
        let box_ = player_bounding_box(position).unwrap();
        // The half-width is `width / 2.0F` widened to double, exactly as
        // `EntityDimensions.makeBoundingBox` computes it, so the edges are the
        // float neighbours of 0.2 and 0.8 rather than those decimals.
        let half_width = f64::from(0.6_f32 / 2.0);
        assert_eq!(box_.min_x, position.x - half_width);
        assert_eq!(box_.max_x, position.x + half_width);
        assert_eq!(box_.min_z, position.z - half_width);
        assert_eq!(box_.min_y, 65.0);
        assert_eq!(box_.max_y, 65.0 + f64::from(1.8_f32));
        assert_eq!(box_.distance_to_sqr(position), 0.0);
    }

    #[test]
    fn the_eye_sits_above_the_feet_by_the_declared_eye_height() {
        let position = Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        };
        let eye = player_eye_position(position).unwrap();
        assert_eq!(eye.x, position.x);
        assert_eq!(eye.z, position.z);
        assert!((eye.y - 66.62).abs() < 1.0E-6);
    }

    #[test]
    fn a_declared_base_value_wins_over_the_registry_default() {
        // `Player.createAttributes` declares attack damage and leaves both
        // interaction ranges to the registry.
        assert_eq!(
            player_attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID).unwrap(),
            1.0
        );
        assert_eq!(
            player_attribute_value(VANILLA_ATTRIBUTE_ENTITY_INTERACTION_RANGE_ID).unwrap(),
            3.0
        );
        assert_eq!(
            player_attribute_value(VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID).unwrap(),
            4.5
        );
    }

    #[test]
    fn entity_reach_is_measured_from_the_eye_to_the_nearest_face() {
        let eye = player_eye_position(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        })
        .unwrap();
        // The buffer the handler passes takes the 3.0 attribute to 6.0, and
        // the comparison is strict, so a box exactly 6.0 away is out.
        let near = player_bounding_box(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 6.7,
        })
        .unwrap();
        let far = player_bounding_box(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 6.9,
        })
        .unwrap();
        assert!(is_within_entity_interaction_range(eye, &near, 3.0, 3.0));
        assert!(!is_within_entity_interaction_range(eye, &far, 3.0, 3.0));
    }

    #[test]
    fn the_default_attack_band_admits_a_box_at_its_far_edge() {
        let eye = player_eye_position(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 0.5,
        })
        .unwrap();
        let range = AttackRange::default_for_player(3.0);
        // Reach is the same 3.0 attribute plus the same 3.0 buffer, but
        // `isInRange` accepts the boundary that the entity-interaction test
        // rejects.
        let edge_z = 0.5 + 6.0 + f64::from(0.6_f32 / 2.0);
        let edge = player_bounding_box(Vec3d {
            x: 0.5,
            y: 65.0,
            z: edge_z,
        })
        .unwrap();
        // The eye is level with the box, so the gap is exactly the 6.0 the
        // band ends at.
        assert_eq!(edge.distance_to_sqr(eye).sqrt(), 6.0);
        assert!(range.is_in_range(eye, true, &edge, 3.0));
        assert!(!is_within_entity_interaction_range(eye, &edge, 3.0, 3.0));
    }

    #[test]
    fn the_default_border_stops_where_the_absolute_clamp_does() {
        let border = WorldBorder::DEFAULT;
        assert!(border.contains_block(BlockPos { x: 0, y: 65, z: 0 }));
        assert!(border.contains_block(BlockPos {
            x: -29_999_984,
            y: 65,
            z: 29_999_983
        }));
        assert!(!border.contains_block(BlockPos {
            x: 29_999_984,
            y: 65,
            z: 0
        }));
        assert!(!border.contains_block(BlockPos {
            x: 0,
            y: 65,
            z: -29_999_985
        }));
    }

    #[test]
    fn a_moving_world_border_uses_level_time_and_freezes_its_remaining_lerp() {
        let border =
            WorldBorder::from_settings(10.0, -10.0, 0.2, 5.0, 5, 300, 100.0, 20, 200.0, 40);
        assert_eq!(border.size_at(40), 100.0);
        assert_eq!(border.size_at(50), 150.0);
        assert_eq!(border.remaining_lerp_time(50), 10);
        assert_eq!(border.distance_to_border_at(10.0, -10.0, 50), 75.0);
        assert_eq!(border.size_at(80), 200.0);
        assert_eq!(border.remaining_lerp_time(80), 0);

        let already_at_target =
            WorldBorder::from_settings(0.0, 0.0, 0.2, 5.0, 5, 300, 100.0, 20, 100.0, 40);
        assert_eq!(already_at_target.size_at(50), 100.0);
        assert_eq!(already_at_target.remaining_lerp_time(40), 0);
        assert_eq!(already_at_target.lerp_target(), 100.0);
    }

    #[test]
    fn player_pose_dimensions_follow_avatar_scale() {
        let crouching = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID;
        let swimming = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SWIMMING_ID;
        assert_eq!(player_pose_dimensions(crouching, 1.0).unwrap(), [0.6, 1.5]);
        assert_eq!(player_pose_dimensions(swimming, 0.5).unwrap(), [0.3, 0.3]);
        for fixed in [
            VANILLA_ENTITY_POSE_SLEEPING_ID,
            VANILLA_ENTITY_POSE_DYING_ID,
        ] {
            assert_eq!(player_pose_dimensions(fixed, 2.0).unwrap(), [0.2, 0.2]);
        }
        let bounds = player_pose_bounding_box(
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            swimming,
            0.5,
        )
        .unwrap();
        assert_eq!(bounds.max_y, 64.0 + f64::from(0.3_f32));
    }
}
