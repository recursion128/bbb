//! The `Sensor` contract and the world one brain tick reads.

use crate::random::GameRandom;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;
use uuid::Uuid;

use super::effect::BrainWorldEffect;
use super::memory::MemoryPath;
use crate::entity::ai::mob::brain_runtime::BrainBodyFacts;
use crate::entity::GroundPathType;

/// `Sensor.DEFAULT_SCAN_RATE` (`Sensor.java:14`), which a sensor naming no rate
/// scans at (`Sensor.java:36-38`).
pub const DEFAULT_SCAN_RATE: i32 = 20;

/// Every field is a state-local read: the collision shape and the face
/// predicates come from the generated 26.1 block-state facts, so a state whose
/// shape is dynamic has no answer here and [`BrainWorld::block_facts`] refuses
/// it rather than reporting a stale one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrainBlockFacts {
    pub is_air: bool,
    pub is_solid_render: bool,
    pub has_fluid: bool,
    pub fluid_is_water: bool,
    pub fluid_supports_frogspawn: bool,
    pub supports_frogspawn: bool,
    pub prefers_frog_jump: bool,
    pub is_water_block: bool,
    pub collision_shape_empty: bool,
    pub collision_face_up_empty: bool,
    pub face_sturdy_up: bool,
}

/// The candidate's own brain is not reachable while this body ticks, so
/// [`BrainWorld::entity_breed_facts`] answers from the projection the mob
/// runtime took before the tick; a candidate outside it is one this world
/// cannot answer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrainBreedFacts {
    pub same_class: bool,
    pub in_love: bool,
    pub panicking: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SensorType {
    NearestLivingEntities,
    NearestPlayers,
    HurtBy,
    FrogTemptations,
    FrogAttackables,
    IsInWater,
    SecondaryPois,
    VillagerHostiles,
    GolemDetected,
}

impl SensorType {
    /// The registry path the type is registered under (`SensorType.java:16-19`,
    /// `:21`, `:38`, `:40`, `:41`).
    pub const fn resource_id(self) -> &'static str {
        match self {
            Self::NearestLivingEntities => "minecraft:nearest_living_entities",
            Self::NearestPlayers => "minecraft:nearest_players",
            Self::HurtBy => "minecraft:hurt_by",
            Self::FrogTemptations => "minecraft:frog_temptations",
            Self::FrogAttackables => "minecraft:frog_attackables",
            Self::IsInWater => "minecraft:is_in_water",
            Self::SecondaryPois => "minecraft:secondary_pois",
            Self::VillagerHostiles => "minecraft:villager_hostiles",
            Self::GolemDetected => "minecraft:golem_detected",
        }
    }
}

/// One cell as a farmer sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FarmCell {
    /// A `CropBlock` at its maximum age.
    RipeCrop,
    GrowingCrop,
    /// Air above farmland.
    EmptyFarmland,
    Other,
}

/// See [`BrainWorld::villager_job_facts`].
#[derive(Debug, Clone, PartialEq)]
pub struct VillagerJobFacts {
    pub profession: &'static str,
    pub xp: i32,
    pub job_site: Option<super::memory::MemoryGlobalPos>,
    pub has_potential_job_site: bool,
}

/// Live age and play partner of a nearby villager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillagerPlayFacts {
    pub baby: bool,
    pub interaction_target: Option<i32>,
}

/// What a world knows about another entity a sensor names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrainEntityFacts {
    /// `Entity.isAlive()`.
    pub alive: bool,
    pub living: bool,
    pub entity_resource_id: &'static str,
    pub uuid: Uuid,
    pub position: Vec3d,
    pub eye_y: f64,
    pub bounding_box: Aabb,
    pub slime_size: Option<i32>,
    pub spectator: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntityTargetingTest {
    pub entity_id: i32,
    /// `TargetingConditions.forCombat()` rather than `forNonCombat()`.
    pub combat: bool,
    pub range: f64,
    pub ignore_line_of_sight: bool,
    pub ignore_invisibility: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrainPlayer {
    /// `Entity.getId()`.
    pub entity_id: i32,
    /// `Player.getMainHandItem().getItem()` as a built-in item type id, `None`
    /// for the empty stack `Player.getMainHandItem()` answers with.
    pub main_hand_item_id: Option<i32>,
    /// `Player.getOffhandItem().getItem()`, the same way.
    pub off_hand_item_id: Option<i32>,
}

pub trait BrainWorld {
    fn game_time(&self) -> i64;

    fn body(&self) -> &BrainBodyFacts;

    fn body_has_passenger(&self, entity_id: i32) -> bool;

    fn entity_breed_facts(&self, entity_id: i32) -> Option<BrainBreedFacts>;

    fn body_can_attack(&self, entity_id: i32) -> bool;

    fn damage_type_in_tag(&self, damage_type: &str, tag_id: &str) -> bool;

    fn entity_type_in_tag(&self, entity_resource_id: &str, tag_id: &str) -> bool;

    fn item_in_tag(&self, item_type_id: i32, tag_id: &str) -> bool;

    fn entity_facts(&self, entity_id: i32) -> Option<BrainEntityFacts>;

    fn nearest_living_entities(&self, range: f64) -> Vec<i32>;

    /// `range` is `follow_distance`, which the caller reads from
    /// [`BrainWorld::body_follow_range`]. `EntitySelector.NO_SPECTATORS` and the
    /// two targeting conditions are the sensor's own filters (`:31`, `:37`,
    /// `:39`); a level with no session list answers nothing, the same branch a
    /// player who logged out takes.
    fn nearest_players(&self, range: f64) -> Vec<i32>;

    /// The sensor walks every player of the level with no distance test of its
    /// own beyond the targeting condition it builds (`:39`), so the named
    /// condition's own range and `EntitySelector.NO_SPECTATORS` are applied by
    /// the caller.
    fn players_within(&self, range: f64) -> Vec<BrainPlayer>;

    fn targeting_test(&self, test: EntityTargetingTest) -> bool;

    fn entity_distance_to_sqr(&self, entity_id: i32) -> Option<f64>;

    fn body_path_distance_to(&self, entity_id: i32, reach_range: i32) -> Option<f32>;

    fn body_prepare_path(&mut self, pos: BlockPos, reach_range: i32) -> Option<MemoryPath>;

    fn body_start_prepared_path(&mut self, speed_modifier: f32);

    fn body_default_random_pos_towards(
        &mut self,
        random: &GameRandom,
        horizontal_dist: i32,
        vertical_dist: i32,
        towards: Vec3d,
        max_xz_radians: f64,
    ) -> Option<Option<Vec3d>>;

    fn body_path(&self) -> Option<MemoryPath>;

    fn body_navigation_is_done(&self) -> bool;

    fn body_navigation_is_stuck(&self) -> bool;

    fn body_pathfinding_malus(&self, path_type: GroundPathType) -> f32;

    fn body_can_fit_at(&self, area: Aabb) -> bool;

    /// `None` is Vanilla's `null` path: `LongJumpToRandomPos.pickCandidate`
    /// jumps on either a `null` path or one that cannot reach, so the two
    /// answers stay apart here.
    fn body_path_reaches(
        &self,
        pos: BlockPos,
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<bool>;

    /// The shared path's original POI and reach status; `None` when no path could be prepared.
    fn body_path_to_pois(
        &self,
        positions: &[BlockPos],
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<(BlockPos, bool)>;

    fn look_at(&mut self, position: Vec3d);

    /// A body action rather than a query, for the same reason
    /// [`BrainWorld::stop_body_navigation`] is: the control belongs to the mob
    /// runtime, not to the brain.
    fn request_body_jump(&mut self);

    fn body_land_random_pos(
        &mut self,
        random: &GameRandom,
        horizontal_dist: i32,
        vertical_dist: i32,
    ) -> Option<Option<Vec3d>>;

    fn body_outdoor_position(&mut self, _random: &GameRandom) -> Option<Vec3d> {
        None
    }

    fn body_has_no_blocks_above(&mut self) -> bool {
        false
    }

    fn body_swim_target_pos(&mut self, random: &GameRandom) -> Option<Option<Vec3d>>;

    /// Delegates to [`LevelCtx::random_next_int`], the level's own
    /// accessor, so there is one stream behind both.
    fn level_random_next_int(&mut self, bound: i32) -> i32 {
        let _ = bound;
        0
    }

    /// `Some(None)` is the empty `Optional` that falls through to the
    /// position getter, `None` a level that cannot answer one of the scan's
    /// reads. Mutable so the world that serves the scan can record it, the
    /// same way [`BrainWorld::body_land_random_pos`] is.
    fn body_closest_water(
        &mut self,
        horizontal_dist: i32,
        vertical_dist: i32,
        two_wide: bool,
    ) -> Option<Option<BlockPos>>;

    /// `None` is a position this level cannot answer -- no chunk, an unknown
    /// state, or a state whose collision shape
    /// is only known to a live world. A sampler refuses on it instead of
    /// judging the cell against a guess.
    fn block_facts(&self, pos: BlockPos) -> Option<BrainBlockFacts>;

    /// `None` for the positions [`BrainWorld::block_facts`] refuses.
    fn block_path_type(&self, pos: BlockPos) -> Option<GroundPathType>;

    fn tongue_target(&self) -> Option<i32>;

    fn set_tongue_target(&mut self, entity_id: Option<i32>);

    fn stop_body_navigation(&mut self);

    fn queue_effect(&mut self, effect: BrainWorldEffect);

    /// `Level.dimension()`, which a `GlobalPos` memory names.
    fn dimension(&self) -> &str {
        ""
    }

    /// `PoiManager.findAllClosestFirstWithType` at `Occupancy.HAS_SPACE`
    /// closest first: `poi_type` names one type,
    /// `None` every `#acquirable_job_site` type.
    fn job_site_candidates(
        &mut self,
        _center: BlockPos,
        _radius: i32,
        _poi_type: Option<&str>,
        _limit: usize,
        _allowed: &dyn Fn(BlockPos) -> bool,
    ) -> Vec<BlockPos> {
        Vec::new()
    }

    /// `PoiManager.getType`.
    fn poi_type_at(&mut self, _pos: BlockPos) -> Option<&'static str> {
        None
    }

    /// `PoiTypeTags.ACQUIRABLE_JOB_SITE` membership.
    fn poi_is_acquirable_job_site(&self, _type_id: &str) -> bool {
        false
    }

    /// `PoiManager.take` of one ticket at `pos`.
    fn poi_take(&mut self, _pos: BlockPos) -> bool {
        false
    }

    /// `PoiManager.release`.
    fn poi_release(&mut self, _pos: BlockPos) {}

    /// `BedBlock.OCCUPIED` of the bed at `pos`; `None` when no bed is there.
    fn bed_occupied(&self, _pos: BlockPos) -> Option<bool> {
        None
    }

    /// `GameRules.MOB_GRIEFING`.
    fn mob_griefing(&self) -> bool {
        false
    }

    /// `Villager.canBreed` of another stored villager.
    fn villager_can_breed(&self, _entity_id: i32) -> bool {
        false
    }

    /// What `HarvestFarmland.validPos` reads of one cell
    fn farm_cell(&self, _pos: BlockPos) -> FarmCell {
        FarmCell::Other
    }

    /// The farmland `SecondaryPoiSensor` finds around `center`
    fn farmland_around(&self, _center: BlockPos) -> Vec<BlockPos> {
        Vec::new()
    }

    /// Whether a sleeping villager lies in the bed at `pos`
    /// (`ValidateNearbyPoi.bedIsOccupiedByVillager`).
    fn villager_sleeps_in(&self, _pos: BlockPos) -> bool {
        false
    }

    /// `LandRandomPos.getPosAway(body, horizontalDist, verticalDist, avoid)`
    /// shaped like
    /// [`BrainWorld::body_land_random_pos`].
    fn body_land_random_pos_away(
        &mut self,
        _random: &GameRandom,
        _horizontal_dist: i32,
        _vertical_dist: i32,
        _avoid: Vec3d,
    ) -> Option<Option<Vec3d>> {
        None
    }

    /// Another living villager's profession, trade experience and job-site
    /// memories, for the behaviours that settle a contested workstation.
    fn villager_job_facts(&self, _entity_id: i32) -> Option<VillagerJobFacts> {
        None
    }

    fn villager_can_reach_job_site(&self, _entity_id: i32, _pos: BlockPos, _reach: i32) -> bool {
        false
    }

    fn villager_play_facts(&self, _entity_id: i32) -> Option<VillagerPlayFacts> {
        None
    }

    fn is_village(&self, _pos: BlockPos) -> bool {
        false
    }

    fn closest_village_section(&self, _pos: BlockPos) -> Option<BlockPos> {
        None
    }

    /// `Level.canSeeSky`.
    fn can_see_sky(&self, _pos: BlockPos) -> bool {
        true
    }

    /// Every home POI within `radius` of `center` whatever its occupancy,
    /// nearest first.
    fn home_pois_within(&mut self, _center: BlockPos, _radius: i32) -> Vec<BlockPos> {
        Vec::new()
    }
}

/// One registered sensor with the counter `Sensor.tick` gates it with.
#[derive(Clone)]
pub(super) struct SensorEntry {
    pub(super) sensor_type: SensorType,
    pub(super) time_to_tick: i64,
}
