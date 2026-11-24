//! `EntityFluidInteraction` (`net/minecraft/world/entity/EntityFluidInteraction.java`)
//! and the generic environment steps of `Entity.baseTick` built on it.

use crate::chunk::ChunkStore;
use crate::world::simulation_level::LevelCtx;
pub(crate) mod water_splash;

#[cfg(test)]
use crate::block_behavior::sweet_berry_bush as sweet_berry_bush_block;
use crate::fluid::above;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_support_facts,
    VanillaBlockSupportDirection, VanillaBlockSupportType,
};
use bbb_protocol::block_types::{
    vanilla_block_type_for_resource_id, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_CAVE_AIR_ID,
    VANILLA_BLOCK_VOID_AIR_ID,
};
use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
};
use bbb_protocol::entity_types;
use bbb_protocol::equipment_slots::{EquipmentSlotType, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::fluid::{
    block_state_blocks_motion, block_state_fluid_state, block_state_is_air, FluidFamily, FluidState,
};
use bbb_server_types::geometry::Aabb;
use std::sync::OnceLock;

use crate::entity::attributes::{AttributeMap, AttributeModifier, AttributeOperation};
use crate::entity::level::fluid_state_tag_membership;
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// `Entity.getTicksRequiredToFreeze()` (`Entity.java:2758-2760`).
pub const TICKS_REQUIRED_TO_FREEZE: i32 = 140;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FluidTag {
    Water,
    Lava,
}

impl FluidTag {
    const fn tag_id(self) -> &'static str {
        match self {
            Self::Water => "minecraft:water",
            Self::Lava => "minecraft:lava",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct FluidTracker(bbb_facts::fluid::FluidTrackerState);

impl FluidTracker {
    fn apply_current_to(self, entity: &mut Entity, scale: f64) {
        entity.set_delta_movement(self.movement_after_current(entity.delta_movement, scale, false));
    }

    fn movement_after_current(self, movement: Vec3d, scale: f64, player: bool) -> Vec3d {
        let [x, y, z] =
            self.0
                .apply_current([movement.x, movement.y, movement.z], scale, player, 1.0);
        Vec3d { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct FluidInteraction {
    water: FluidTracker,
    lava: FluidTracker,
}

impl FluidInteraction {
    pub fn movement_after_current(
        &self,
        tag: FluidTag,
        movement: Vec3d,
        scale: f64,
        player: bool,
    ) -> Vec3d {
        self.tracker(tag)
            .movement_after_current(movement, scale, player)
    }
    const fn tracker(&self, tag: FluidTag) -> &FluidTracker {
        match tag {
            FluidTag::Water => &self.water,
            FluidTag::Lava => &self.lava,
        }
    }

    /// `EntityFluidInteraction.getFluidHeight` (`:139-142`).
    pub const fn fluid_height(&self, tag: FluidTag) -> f64 {
        self.tracker(tag).0.height
    }

    /// `:144-146`.
    pub fn is_in_fluid(&self, tag: FluidTag) -> bool {
        self.fluid_height(tag) > 0.0
    }

    /// `:148-151`.
    pub const fn is_eye_in_fluid(&self, tag: FluidTag) -> bool {
        self.tracker(tag).0.eyes_inside
    }
}

/// `BlockGetter.getBlockState` over the level's stored chunks.
fn block_state(chunks: &ChunkStore, pos: BlockPos) -> i32 {
    crate::world::stored_block_state(chunks, pos)
}

fn fluid_in_tag(chunks: &ChunkStore, fluid: FluidState, tag: &str) -> bool {
    let fluid = fluid.identity().resource_id();
    chunks.registry_tags().map_or_else(
        || {
            crate::entity::EntityRegistryTagView::Builtin26_1.contains(
                "minecraft:fluid",
                tag,
                fluid,
            )
        },
        |tags| tags.contains("minecraft:fluid", tag, fluid),
    )
}

pub fn scan_body_fluid_interaction(
    bounding_box: Aabb,
    eye: Vec3d,
    vehicle: Option<&Entity>,
    chunks: &ChunkStore,
) -> Option<FluidInteraction> {
    scan_body_fluid_interaction_with_current(bounding_box, eye, vehicle, chunks, true)
}

pub fn scan_body_fluid_interaction_with_current(
    bounding_box: Aabb,
    eye: Vec3d,
    vehicle: Option<&Entity>,
    chunks: &ChunkStore,
    ignore_current: bool,
) -> Option<FluidInteraction> {
    let mut area = bounding_box.inflate(-FLUID_BOX_MARGIN, -FLUID_BOX_MARGIN, -FLUID_BOX_MARGIN);
    if let Some(vehicle) = vehicle {
        if let Some(data) = crate::entity::boat::boat(&vehicle.data) {
            if !data.is_under_water() {
                let rim = vehicle.bounding_box().max_y;
                if rim >= area.max_y {
                    return Some(FluidInteraction::default());
                }
                area.min_y = area.min_y.max(rim);
            }
        }
    }
    scan_box(bounding_box.min_y, area, eye, chunks, ignore_current)
}

/// `Entity.getFluidInteractionBox`'s deflation (`Entity.java:4000-4001`).
const FLUID_BOX_MARGIN: f64 = bbb_facts::fluid::FLUID_BOX_MARGIN;

/// `Entity.updateFluidInteraction`'s three current scales (`Entity.java:1568`,
/// `:1572`).
pub(crate) const WATER_CURRENT_SCALE: f64 = bbb_facts::fluid::WATER_CURRENT_SCALE;
pub(crate) const FAST_LAVA_CURRENT_SCALE: f64 = bbb_facts::fluid::FAST_LAVA_CURRENT_SCALE;
pub(crate) const LAVA_CURRENT_SCALE: f64 = bbb_facts::fluid::LAVA_CURRENT_SCALE;

/// `EntityFluidInteraction.update` (`:32-89`), as a fresh interaction: every
/// tracker is reset first (`:33`), so the previous value never survives.
///
/// `None` is the installed registry generation not resolving a block state or
/// a fluid tag the scan reads; the caller fails closed rather than reporting a
/// partial scan as an answer.
fn scan(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    ignore_current: bool,
) -> Option<FluidInteraction> {
    let bounding_box = entity.bounding_box();
    scan_box(
        bounding_box.min_y,
        bounding_box.inflate(-FLUID_BOX_MARGIN, -FLUID_BOX_MARGIN, -FLUID_BOX_MARGIN),
        Vec3d {
            y: entity.position.y + f64::from(entity.eye_height()),
            ..entity.position
        },
        level.chunks(),
        ignore_current,
    )
}

fn scan_box(
    entity_y: f64,
    interaction_box: Aabb,
    eye: Vec3d,
    chunks: &ChunkStore,
    ignore_current: bool,
) -> Option<FluidInteraction> {
    let x0 = interaction_box.min_x.floor() as i32;
    let z0 = interaction_box.min_z.floor() as i32;
    let x1 = interaction_box.max_x.ceil() as i32 - 1;
    let z1 = interaction_box.max_z.ceil() as i32 - 1;
    if !scanned_chunks_loaded(chunks, x0 - 1, z0 - 1, x1 + 1, z1 + 1) {
        return Some(FluidInteraction::default());
    }
    let scan = bbb_facts::fluid::scan_fluid_contact(
        interaction_box,
        entity_y,
        [eye.x, eye.y, eye.z],
        ignore_current,
        |[x, y, z]| {
            let pos = BlockPos { x, y, z };
            let fluid = fluid_state_at(chunks, pos)?;
            if fluid.is_empty() {
                return Some(None);
            }
            let height = f64::from(fluid_height_at(chunks, pos, fluid)?);
            let Some(tag) = tracker_tag(chunks, fluid) else {
                return Some(None);
            };
            let kind = match tag {
                FluidTag::Water => bbb_facts::fluid::FluidKind::Water,
                FluidTag::Lava => bbb_facts::fluid::FluidKind::Lava,
            };
            Some(Some((kind, height)))
        },
        |[x, y, z], _| {
            let pos = BlockPos { x, y, z };
            let fluid = fluid_state_at(chunks, pos)?;
            flow(chunks, pos, fluid)
        },
    )?;
    Some(FluidInteraction {
        water: FluidTracker(scan.water),
        lava: FluidTracker(scan.lava),
    })
}

/// The loaded-chunk half of `EntityFluidInteraction.hasFluidAndLoaded`
/// (`:91-119`). The per-section `hasFluid()` half only short-circuits a scan
/// that would find nothing, so omitting it cannot change a tracker.
fn scanned_chunks_loaded(chunks: &ChunkStore, x0: i32, z0: i32, x1: i32, z1: i32) -> bool {
    for chunk_z in section_coord(z0)..=section_coord(z1) {
        for chunk_x in section_coord(x0)..=section_coord(x1) {
            if !chunks.is_ticking_full(chunk_x, chunk_z) {
                return false;
            }
        }
    }
    true
}

/// `SectionPos.blockToSectionCoord(int)` (`SectionPos.java:80-82`).
const fn section_coord(block: i32) -> i32 {
    block >> 4
}

/// `BlockGetter.getFluidState` for a state this crate can resolve; `None` is a
/// block-state id outside the generated 26.1 registry.
pub(super) fn fluid_state_at(chunks: &ChunkStore, pos: BlockPos) -> Option<FluidState> {
    let state_id = block_state(chunks, pos);
    if vanilla_block_state_block_type_id(state_id).is_none() {
        chunks.content_registry().block_by_state_id(state_id)?;
        return Some(FluidState::Empty);
    }
    Some(block_state_fluid_state(state_id))
}

pub(super) fn fluid_height_at(
    chunks: &ChunkStore,
    pos: BlockPos,
    state: FluidState,
) -> Option<f32> {
    let sample = state.to_facts_sample();
    bbb_facts::fluid::checked_fluid_height_f32(sample, {
        if sample.is_none() {
            None
        } else {
            let over = fluid_state_at(chunks, above(pos))?;
            Some(
                over.family()
                    .map(bbb_server_types::fluid::FluidFamily::to_facts_kind),
            )
        }
    })
}

/// `EntityFluidInteraction.getTrackerFor` (`:121-130`): the registered tag this
/// fluid `is`, or `None` when it is in neither.
fn tracker_tag(chunks: &ChunkStore, fluid: FluidState) -> Option<FluidTag> {
    [FluidTag::Water, FluidTag::Lava]
        .into_iter()
        .find(|&tag| fluid_in_tag(chunks, fluid, tag.tag_id()))
}

fn to_packet_direction(face: bbb_facts::collision::BlockFace) -> Direction {
    match face {
        bbb_facts::collision::BlockFace::Down => Direction::Down,
        bbb_facts::collision::BlockFace::Up => Direction::Up,
        bbb_facts::collision::BlockFace::North => Direction::North,
        bbb_facts::collision::BlockFace::South => Direction::South,
        bbb_facts::collision::BlockFace::West => Direction::West,
        bbb_facts::collision::BlockFace::East => Direction::East,
    }
}

fn flow(chunks: &ChunkStore, pos: BlockPos, state: FluidState) -> Option<[f64; 3]> {
    let sample = state.to_facts_sample()?;
    let family = state.family()?;
    bbb_facts::fluid::fluid_flow_vector(
        [pos.x, pos.y, pos.z],
        sample,
        |[x, y, z]| {
            let cell_pos = BlockPos { x, y, z };
            let fluid = fluid_state_at(chunks, cell_pos)?;
            Some(bbb_facts::fluid::FluidFlowCell {
                fluid: fluid.to_facts_sample(),
                blocks_motion: block_state_blocks_motion(block_state(chunks, cell_pos)),
            })
        },
        |[x, y, z], face| {
            solid_face(
                chunks,
                BlockPos { x, y, z },
                to_packet_direction(face),
                family,
            )
        },
    )
}

fn solid_face(
    chunks: &ChunkStore,
    pos: BlockPos,
    direction: Direction,
    family: FluidFamily,
) -> Option<bool> {
    let state_id = block_state(chunks, pos);
    let Some(block_type_id) = vanilla_block_state_block_type_id(state_id) else {
        let (block, _) = chunks.content_registry().block_by_state_id(state_id)?;
        return Some(block.full_cube_collision);
    };
    if block_state_fluid_state(state_id).is_same_family(family) {
        return Some(false);
    }
    if is_ice_block(block_type_id) {
        return Some(false);
    }
    Some(
        vanilla_block_state_support_facts(state_id).is_some_and(|facts| {
            facts.face_sturdy(support_direction(direction), VanillaBlockSupportType::Full)
        }),
    )
}

fn is_ice_block(block_type_id: i32) -> bool {
    static IDS: OnceLock<Vec<i32>> = OnceLock::new();
    IDS.get_or_init(|| {
        ["minecraft:ice", "minecraft:frosted_ice"]
            .iter()
            .map(|resource_id| {
                vanilla_block_type_for_resource_id(resource_id)
                    .expect("Vanilla 26.1 registers the ice blocks")
                    .id
            })
            .collect()
    })
    .contains(&block_type_id)
}

const fn support_direction(direction: Direction) -> VanillaBlockSupportDirection {
    match direction {
        Direction::Down => VanillaBlockSupportDirection::Down,
        Direction::Up => VanillaBlockSupportDirection::Up,
        Direction::North => VanillaBlockSupportDirection::North,
        Direction::South => VanillaBlockSupportDirection::South,
        Direction::West => VanillaBlockSupportDirection::West,
        Direction::East => VanillaBlockSupportDirection::East,
    }
}

/// The concrete leaves of the classes that answer `isPushedByFluid()` false:
/// `AgeableWaterCreature` (dolphin, squid, glow_squid), `AbstractFish`
/// (`WaterAnimal`'s only subclass tree), `AbstractNautilus`, `Axolotl`,
/// `Frog` and `Turtle`.
pub(crate) fn is_pushed_by_fluid(entity: &Entity) -> bool {
    match entity.data() {
        EntityData::Arrow(data) => return !data.in_ground(),
        EntityData::SpectralArrow(data) => return !data.in_ground(),
        EntityData::Trident(data) => return !data.in_ground(),
        _ => {}
    }
    let entity_type_id = entity.entity_type_id();
    if entity_type_id == entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID {
        return !entity.is_swimming();
    }
    !matches!(
        entity_type_id,
        entity_types::VANILLA_ENTITY_TYPE_DOLPHIN_ID
            | entity_types::VANILLA_ENTITY_TYPE_SQUID_ID
            | entity_types::VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
            | entity_types::VANILLA_ENTITY_TYPE_COD_ID
            | entity_types::VANILLA_ENTITY_TYPE_SALMON_ID
            | entity_types::VANILLA_ENTITY_TYPE_PUFFERFISH_ID
            | entity_types::VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID
            | entity_types::VANILLA_ENTITY_TYPE_TADPOLE_ID
            | entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
            | entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
            | entity_types::VANILLA_ENTITY_TYPE_AXOLOTL_ID
            | entity_types::VANILLA_ENTITY_TYPE_FROG_ID
            | entity_types::VANILLA_ENTITY_TYPE_TURTLE_ID
    )
}

#[cfg(test)]
pub(crate) fn update_fluid_interaction(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    let _ = update_fluid_interaction_checked(entity, level);
}

pub(crate) fn update_fluid_interaction_checked(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
) -> Option<()> {
    let interaction = scan(entity, level, !is_pushed_by_fluid(entity))?;
    begin_fluid_interaction(entity, interaction);
    finish_fluid_interaction(entity, level)
}

pub(crate) fn update_fluid_interaction_in_level(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let vehicle = match store.vehicle(entity.id()) {
        Some(id) => Some(store.get(id)?),
        None => None,
    };
    let interaction = scan_body_fluid_interaction_with_current(
        entity.bounding_box(),
        Vec3d {
            y: entity.position.y + f64::from(entity.eye_height()),
            ..entity.position
        },
        vehicle,
        level.chunks(),
        !is_pushed_by_fluid(entity),
    )?;
    begin_fluid_interaction(entity, interaction);
    let first_tick = entity.is_first_tick();
    if interaction.is_in_fluid(FluidTag::Water)
        && !entity.was_touching_water
        && !first_tick
        && !matches!(entity.data(), EntityData::ExperienceOrb(_))
    {
        let (movement, controlled) = water_splash::controller_movement(entity, store, level)?;
        let (volume, pitch) = water_splash::body_water_splash(movement, controlled, store.random());
        if !entity.is_silent() {
            store.play_entity_sound(crate::entity::store::PendingEntitySound {
                position: entity.position(),
                resource_id: water_splash::sound(entity, volume).to_owned(),
                source: crate::entity::living::living_sound_source(entity),
                volume,
                pitch,
            });
        }
        level.game_event_at(
            crate::level_event::VanillaGameEvent::SPLASH,
            entity.position(),
            Some(entity.id()),
        );
    }
    finish_fluid_interaction(entity, level)
}

fn begin_fluid_interaction(entity: &mut Entity, interaction: FluidInteraction) {
    entity.fluid_interaction = interaction;
    if interaction.is_in_fluid(FluidTag::Water) {
        entity.reset_fall_distance();
    }
}

fn finish_fluid_interaction(entity: &mut Entity, level: &LevelCtx<'_, '_>) -> Option<()> {
    let interaction = entity.fluid_interaction;
    let in_water = interaction.is_in_fluid(FluidTag::Water);
    let in_lava = interaction.is_in_fluid(FluidTag::Lava);
    let pushed = is_pushed_by_fluid(entity);
    entity.was_touching_water = in_water;
    if pushed {
        if in_water {
            interaction
                .tracker(FluidTag::Water)
                .apply_current_to(entity, WATER_CURRENT_SCALE);
        }
        if in_lava {
            let scale = if level.fast_lava_checked()? {
                FAST_LAVA_CURRENT_SCALE
            } else {
                LAVA_CURRENT_SCALE
            };
            interaction
                .tracker(FluidTag::Lava)
                .apply_current_to(entity, scale);
        }
    }
    Some(())
}

///
/// `isPassenger()` reads the riding relation, which lives in [`crate::entity::store`]
/// and is not handed to the base tick; it is answered false here, which is the
/// only arm an entity that is not a passenger can take.
pub(crate) fn update_swimming(
    entity: &mut Entity,
    store: Option<&crate::entity::EntityStore>,
    level: &LevelCtx<'_, '_>,
) {
    // Drowned.java:231-255.
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID {
        let swimming = entity.is_under_water()
            && matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if !data.is_no_ai())
            && store.is_some_and(|store| {
                store.mob_ai(entity.id()).is_some_and(|runtime| {
                    runtime
                        .drowned_swim_intent(entity, store, level)
                        .wants_to_swim
                })
            });
        entity.set_swimming(swimming);
        return;
    }
    if let Some(swimming) = body_swimming_state(
        entity.is_swimming(),
        entity.is_sprinting(),
        entity.is_in_water(),
        entity.was_eye_in_water,
        entity.block_position(),
        level.chunks(),
    ) {
        entity.set_swimming(swimming);
    }
}

pub fn body_swimming_state(
    was_swimming: bool,
    sprinting: bool,
    in_water: bool,
    was_eye_in_water: bool,
    feet: BlockPos,
    chunks: &ChunkStore,
) -> Option<bool> {
    if !sprinting || !in_water {
        return Some(false);
    }
    if was_swimming {
        return Some(true);
    }
    if !was_eye_in_water {
        return Some(false);
    }
    let fluid = fluid_state_at(chunks, feet)?;
    Some(fluid_in_tag(chunks, fluid, FluidTag::Water.tag_id()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FluidContact {
    Known(bool),
    Unknown,
}

impl FluidContact {
    pub(crate) const fn known(self) -> Option<bool> {
        match self {
            Self::Known(in_water) => Some(in_water),
            Self::Unknown => None,
        }
    }
}

pub(crate) fn fluid_contact(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    tag: FluidTag,
) -> FluidContact {
    let tag_id = tag.tag_id();
    match bbb_facts::fluid::aabb_in_fluid(entity.bounding_box(), |[x, y, z]| {
        let pos = BlockPos { x, y, z };
        let state_id = level.block_state(pos);
        if vanilla_block_state_block_type_id(state_id).is_none() {
            level.content_registry().block_by_state_id(state_id)?;
            return Some(None);
        }
        let fluid = block_state_fluid_state(state_id);
        if !fluid_state_tag_membership(level, fluid, tag_id) {
            return Some(None);
        }
        let above_id = level.block_state(BlockPos { y: y + 1, ..pos });
        if vanilla_block_state_block_type_id(above_id).is_none() {
            level.content_registry().block_by_state_id(above_id)?;
        }
        let height = bbb_facts::fluid::fluid_height_f32(
            fluid.to_facts_sample(),
            block_state_fluid_state(above_id).family() == fluid.family(),
        );
        Some(Some(f64::from(height)))
    }) {
        Some(in_water) => FluidContact::Known(in_water),
        None => FluidContact::Unknown,
    }
}

impl Entity {
    /// `Entity.isEyeInFluid(TagKey)` (`Entity.java:1644-1646`).
    pub const fn is_eye_in_fluid(&self, tag: FluidTag) -> bool {
        self.fluid_interaction.is_eye_in_fluid(tag)
    }

    /// `Entity.getFluidHeight(TagKey)` (`Entity.java:3596-3598`).
    pub const fn fluid_height(&self, tag: FluidTag) -> f64 {
        self.fluid_interaction.fluid_height(tag)
    }

    pub fn is_in_lava(&self) -> bool {
        !self.first_tick && self.fluid_interaction.is_in_fluid(FluidTag::Lava)
    }

    /// `Entity.isInWater()` (`Entity.java:1505-1507`).
    pub const fn is_in_water(&self) -> bool {
        self.was_touching_water
    }

    /// `Entity.isUnderWater()` (`Entity.java:1522-1524`).
    pub const fn is_under_water(&self) -> bool {
        self.was_eye_in_water && self.is_in_water()
    }

    /// `Entity.isSprinting()` (`Entity.java:2635-2637`).
    pub const fn is_sprinting(&self) -> bool {
        self.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING)
    }

    /// `Entity.isSwimming()` (`Entity.java:2643-2645`).
    pub const fn is_swimming(&self) -> bool {
        self.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING)
    }

    pub fn set_swimming(&mut self, swimming: bool) {
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SWIMMING, swimming);
    }

    /// `Entity.isInPowderSnow` (`Entity.java:286`), cleared by every base tick
    /// and set again by the powder-snow block's own entity-inside hook.
    pub const fn is_in_powder_snow(&self) -> bool {
        self.is_in_powder_snow
    }

    pub fn set_is_in_powder_snow(&mut self, value: bool) {
        self.is_in_powder_snow = value;
    }

    /// `Entity.wasInPowderSnow` (`Entity.java:287`).
    pub const fn was_in_powder_snow(&self) -> bool {
        self.was_in_powder_snow
    }

    pub(crate) fn is_in_rain(&self, level: &LevelCtx<'_, '_>) -> bool {
        let pos = self.block_position();
        level.is_raining_at(pos)
            || level.is_raining_at(BlockPos {
                y: self.bounding_box.max_y.floor() as i32,
                ..pos
            })
    }

    pub fn is_fully_frozen(&self) -> bool {
        self.ticks_frozen >= TICKS_REQUIRED_TO_FREEZE
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KnownContact {
    Known(bool),
    Unknown,
}

#[cfg(test)]
#[derive(Debug, Default)]
struct InsideBlockScan {
    powder_snow: bool,
    cobweb: bool,
    cactus: bool,
    sweet_berry: bool,
    sweet_berry_age_nonzero: bool,
    honey: Vec<BlockPos>,
}

#[cfg(test)]
fn scan_inside_blocks(entity: &Entity, level: &LevelCtx<'_, '_>) -> Option<InsideBlockScan> {
    let powder_snow = vanilla_block_type_for_resource_id("minecraft:powder_snow")?;
    let cobweb = vanilla_block_type_for_resource_id("minecraft:cobweb")?;
    let cactus = vanilla_block_type_for_resource_id("minecraft:cactus")?;
    let sweet_berry = vanilla_block_type_for_resource_id("minecraft:sweet_berry_bush")?;
    let honey = vanilla_block_type_for_resource_id("minecraft:honey_block")?;
    let bb = entity.bounding_box();
    let epsilon = f64::from(1.0e-5_f32);
    let min_x = (bb.min_x + epsilon).floor() as i32;
    let min_y = (bb.min_y + epsilon).floor() as i32;
    let min_z = (bb.min_z + epsilon).floor() as i32;
    let max_x = (bb.max_x - epsilon).ceil() as i32 - 1;
    let max_y = (bb.max_y - epsilon).ceil() as i32 - 1;
    let max_z = (bb.max_z - epsilon).ceil() as i32 - 1;
    let mut unknown = false;
    let mut scan = InsideBlockScan::default();
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let pos = BlockPos { x, y, z };
                let state = level.block_state(pos);
                let Some(block_type) = vanilla_block_state_block_type_id(state) else {
                    unknown = true;
                    continue;
                };
                if block_type == powder_snow.id {
                    scan.powder_snow = true;
                } else if block_type == cobweb.id {
                    scan.cobweb = true;
                } else if block_type == cactus.id {
                    scan.cactus = true;
                } else if block_type == sweet_berry.id {
                    scan.sweet_berry = true;
                    if sweet_berry_bush_block::age(state).is_some_and(|age| age != 0) {
                        scan.sweet_berry_age_nonzero = true;
                    }
                } else if block_type == honey.id {
                    scan.honey.push(pos);
                }
            }
        }
    }
    (!unknown).then_some(scan)
}

pub(crate) fn can_freeze(entity: &Entity, level: &LevelCtx<'_, '_>) -> KnownContact {
    let armor_items = VANILLA_EQUIPMENT_SLOTS.iter().filter_map(|slot| {
        if !matches!(
            slot.slot_type,
            EquipmentSlotType::HumanoidArmor | EquipmentSlotType::AnimalArmor
        ) {
            return None;
        }
        entity.living()?.item_by_slot(slot.slot)?.item_resource_id()
    });
    match can_body_freeze(
        false,
        entity.data().entity_resource_id(),
        armor_items,
        |registry, tag, item| level.registry_tag_membership(registry, tag, item),
    ) {
        Some(value) => KnownContact::Known(value),
        None => KnownContact::Unknown,
    }
}

pub fn can_body_freeze<'a>(
    spectator: bool,
    entity_type: &str,
    armor_items: impl IntoIterator<Item = &'a str>,
    tag_lookup: impl Fn(&str, &str, &str) -> bool,
) -> Option<bool> {
    // Skeleton.java:103 overrides canFreeze to false.
    if spectator || entity_type == "minecraft:skeleton" {
        return Some(false);
    }
    for item in armor_items {
        if tag_lookup("minecraft:item", "minecraft:freeze_immune_wearables", item) {
            return Some(false);
        }
    }
    Some(!tag_lookup(
        "minecraft:entity_type",
        "minecraft:freeze_immune_entity_types",
        entity_type,
    ))
}

#[cfg(test)]
pub(crate) fn apply_inside_block_environment(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
) -> bool {
    let Some(inside) = scan_inside_blocks(entity, level) else {
        return false;
    };
    entity.is_in_powder_snow = inside.powder_snow;
    if inside.cobweb {
        entity.make_stuck_in_block(cobweb_stuck_multiplier(entity));
    }
    if inside.sweet_berry && sweet_berry_entity_inside_applies(entity) {
        entity.make_stuck_in_block(sweet_berry_stuck_multiplier());
    }
    for pos in inside.honey {
        if honey_is_sliding_down(pos, entity) {
            honey_do_slide_movement(entity);
        }
    }
    if entity.is_in_water() || entity.is_in_powder_snow || entity.is_in_rain(level) {
        entity.clear_fire();
    }
    if entity.is_in_powder_snow {
        match can_freeze(entity, level) {
            KnownContact::Unknown => return false,
            KnownContact::Known(true) => {
                let next = (entity.ticks_frozen + 1).min(TICKS_REQUIRED_TO_FREEZE);
                entity.set_ticks_frozen(next);
            }
            KnownContact::Known(false) => {}
        }
    }
    true
}

#[cfg(test)]
pub(super) fn sweet_berry_entity_inside_applies(entity: &Entity) -> bool {
    crate::block_behavior::sweet_berry_bush::inside_applies(
        entity.living().is_some(),
        entity.entity_type_id(),
    )
}

#[cfg(test)]
pub(super) fn sweet_berry_stuck_multiplier() -> Vec3d {
    crate::block_behavior::sweet_berry_bush::stuck_multiplier()
}

#[cfg(test)]
pub(super) fn cobweb_stuck_multiplier(entity: &Entity) -> Vec3d {
    let weaving = entity
        .living()
        .is_some_and(|living| living.active_effect("minecraft:weaving").is_some());
    crate::block_behavior::inside_contact::cobweb_stuck_multiplier(weaving)
}

#[cfg(test)]
pub(super) fn honey_is_sliding_down(pos: BlockPos, entity: &Entity) -> bool {
    crate::block_behavior::honey::is_sliding_down(
        pos,
        entity.position(),
        entity.bb_width(),
        entity.on_ground(),
        entity.delta_movement(),
    )
}

#[cfg(test)]
pub(super) fn honey_do_slide_movement(entity: &mut Entity) {
    entity.set_delta_movement(crate::block_behavior::honey::slide_movement(
        entity.delta_movement(),
    ));
    entity.reset_fall_distance();
}

pub fn tick_body_freeze(
    ticks: i32,
    in_powder_snow: bool,
    can_freeze: impl FnOnce() -> Option<bool>,
) -> Option<i32> {
    if !in_powder_snow || !can_freeze()? {
        Some(ticks.wrapping_sub(2).max(0))
    } else {
        Some(ticks)
    }
}

pub fn body_freeze_damage_due(
    ticks: i32,
    tick_count: i32,
    can_freeze: impl FnOnce() -> Option<bool>,
) -> Option<bool> {
    if tick_count % 40 == 0 && ticks >= TICKS_REQUIRED_TO_FREEZE {
        can_freeze()
    } else {
        Some(false)
    }
}

pub fn update_body_frost(attributes: &mut AttributeMap, ticks: i32, legacy_block_is_air: bool) {
    if !legacy_block_is_air && ticks > 0 {
        let Some(speed) = attributes.get_instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID) else {
            return;
        };
        speed.remove_modifier("minecraft:powder_snow");
        let percent = ticks.min(TICKS_REQUIRED_TO_FREEZE) as f32 / TICKS_REQUIRED_TO_FREEZE as f32;
        let amount = f64::from(-0.05_f32 * percent);
        speed
            .add_transient_modifier(AttributeModifier {
                id: "minecraft:powder_snow".to_string(),
                amount,
                operation: AttributeOperation::AddValue,
            })
            .expect("the frost modifier was removed");
    } else if let Some(speed) = attributes.instance_mut(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID) {
        speed.remove_modifier("minecraft:powder_snow");
    }
}

pub(crate) fn apply_living_freeze_suffix(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
) -> KnownContact {
    let query = |entity: &Entity| match can_freeze(entity, level) {
        KnownContact::Known(value) => Some(value),
        KnownContact::Unknown => None,
    };
    let Some(ticks) = tick_body_freeze(entity.ticks_frozen, entity.is_in_powder_snow, || {
        query(entity)
    }) else {
        return KnownContact::Unknown;
    };
    entity.set_ticks_frozen(ticks);
    let legacy_block_is_air = if ticks > 0 {
        block_state_is_air(level.block_state(entity.on_pos_legacy_in(level)))
    } else {
        true
    };
    if let Some(living) = entity.living_mut() {
        update_body_frost(living.attributes_mut(), ticks, legacy_block_is_air);
    }
    match body_freeze_damage_due(ticks, entity.tick_count, || query(entity)) {
        Some(value) => KnownContact::Known(value),
        None => KnownContact::Unknown,
    }
}

pub(crate) fn is_in_wall(entity: &Entity, level: &LevelCtx<'_, '_>) -> KnownContact {
    match body_is_in_wall(
        entity.no_physics,
        Vec3d {
            x: entity.position.x,
            y: entity.position.y + f64::from(entity.eye_height()),
            z: entity.position.z,
        },
        entity.bb_width(),
        |pos| level.block_state(pos),
        |pos| level.block_collision_boxes(pos),
    ) {
        Some(value) => KnownContact::Known(value),
        None => KnownContact::Unknown,
    }
}

pub fn body_is_in_wall(
    no_physics: bool,
    eye: Vec3d,
    width: f32,
    block_state: impl Fn(BlockPos) -> i32,
    collision_boxes: impl Fn(BlockPos) -> Vec<Aabb>,
) -> Option<bool> {
    if no_physics {
        return Some(false);
    }
    let check_width = f64::from(width * 0.8_f32);
    let eye_bb = Aabb::of_size(eye, check_width, 1.0e-6, check_width);
    let min_x = eye_bb.min_x.floor() as i32;
    let min_y = eye_bb.min_y.floor() as i32;
    let min_z = eye_bb.min_z.floor() as i32;
    let max_x = eye_bb.max_x.floor() as i32;
    let max_y = eye_bb.max_y.floor() as i32;
    let max_z = eye_bb.max_z.floor() as i32;
    let mut unknown = false;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let pos = BlockPos { x, y, z };
                let state = block_state(pos);
                let Some(block_type) = vanilla_block_state_block_type_id(state) else {
                    unknown = true;
                    continue;
                };
                if is_air_block(block_type) {
                    continue;
                }
                let Some(facts) = vanilla_block_state_support_facts(state) else {
                    unknown = true;
                    continue;
                };
                if !facts.suffocating() {
                    continue;
                }
                for shape in collision_boxes(pos) {
                    let world = Aabb {
                        min_x: shape.min_x + f64::from(pos.x),
                        min_y: shape.min_y + f64::from(pos.y),
                        min_z: shape.min_z + f64::from(pos.z),
                        max_x: shape.max_x + f64::from(pos.x),
                        max_y: shape.max_y + f64::from(pos.y),
                        max_z: shape.max_z + f64::from(pos.z),
                    };
                    if world.intersects(eye_bb) {
                        return Some(true);
                    }
                }
            }
        }
    }
    if unknown {
        None
    } else {
        Some(false)
    }
}

const fn is_air_block(block_type_id: i32) -> bool {
    matches!(
        block_type_id,
        VANILLA_BLOCK_AIR_ID | VANILLA_BLOCK_CAVE_AIR_ID | VANILLA_BLOCK_VOID_AIR_ID
    )
}

impl Entity {
    /// `Entity.isCrouching()` (`Entity.java:2631-2633`).
    pub const fn is_crouching(&self) -> bool {
        self.pose_id == VANILLA_ENTITY_POSE_CROUCHING_ID
    }
}
