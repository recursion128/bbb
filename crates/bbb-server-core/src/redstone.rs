//! What a world answers when something asks it for a redstone signal.
//!
//!
//! # Why this is not in `Level`
//!
//!
//! # The two names that are not what they sound like
//!
//! `getSignal` is not "weak power" and `getDirectSignal` is not "strong
//! power" in the sense a wiki uses. `SignalGetter.getSignal` (`:65-69`) asks
//! the state at the position for its own signal and then, **if that state is
//! a redstone conductor**, folds `getDirectSignalTo(pos)` in with a `max` —
//! that fold is what a "powered block" is. A port that swaps the two names
//! ends up with a stone block beside a lever that conducts nothing.
//!
//! # `isRedstoneConductor` is a generated per-state bit, not a shape test
//!
//!
//!
//! So [`is_redstone_conductor`] reads the generated per-state bit and never
//! reconstructs the predicate; [`SignalContext::is_conductor_at`] adds the one
//! live exception, an open shulker box.
//!
//! # Dispatch
//!
//!
//! # `shouldSignal` is a context, not a constant
//!
//!
//! A mutable field on a process-wide singleton is not modelled here as one.
//! [`SignalContext`] carries it beside the block reader, so the suppression is
//! a dynamic extent a caller opens on purpose and every question in this module
//! states which one it is asking in. A port that treated it as a constant
//! `true` would give every wire its neighbour's power without the decay.
//!
//! `DiodeBlock` is shared by the repeater and comparator; powered comparator
//! output is projected from its live block entity through [`SignalContext`].

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_state_support_facts;
use bbb_protocol::block_state_types::VanillaBlockSupportType as SupportType;
use bbb_protocol::block_types::{
    vanilla_block_type, VanillaBlockAnalogOutputOverride, VanillaBlockRedstoneSignalOverride,
    VANILLA_BLOCK_REDSTONE_BLOCK_ID, VANILLA_BLOCK_REDSTONE_WIRE_ID,
};
use bbb_protocol::packets::{BlockPos, Direction};

use crate::block_behavior::{
    beehive, cake, composter, copper_bulb, copper_golem_statue, diode, redstone_torch,
    redstone_wire, string_property, BlockStateAt,
};
use bbb_server_types::block_queries::{
    block_type_of, bool_property, int_property, DIRECTION_VALUES,
};
use bbb_server_types::directions::relative;

/// `Redstone.SIGNAL_MAX` (`Redstone.java:7`), the value every early return in
/// [`get_direct_signal_to`] and [`get_best_neighbor_signal`] compares against.
pub const SIGNAL_MAX: i32 = 15;

/// `SignalGetter.DIRECTIONS` (`SignalGetter.java:11`), which is
/// `Direction.values()` in enum-declaration order (`Direction.java:33-38`):
/// down, up, north, south, west, east.
///
/// It is neither [`crate::neighbor_updater::UPDATE_ORDER`] (west, east, down,
/// up, north, south — `NeighborUpdater.java:18`) nor
/// [`crate::neighbor_updater::UPDATE_SHAPE_ORDER`] (west, east, north, south,
/// down, up — `BlockBehaviour.java:86-88`), and the difference is observable:
/// [`get_best_neighbor_signal`] stops at the first side answering
/// [`SIGNAL_MAX`], so the order decides how many neighbours are read.
///
/// The array itself is [`crate::block_behavior::DIRECTION_VALUES`], because
/// `SignalGetter.DIRECTIONS` and `LeavesBlock`'s `Direction.values()` walk are
/// the same six constants in the same order and there is no second copy of
/// them here.
pub const DIRECTIONS: [Direction; 6] = DIRECTION_VALUES;

/// `BlockStateProperties.POWER` as `RedStoneWireBlock.POWER`
/// (`RedStoneWireBlock.java:50`), read by [`get_control_input_signal`].
const POWER: &str = "power";

/// Which of `BlockBehaviour`'s three redstone members was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SignalMember {
    IsSignalSource,
    GetSignal,
    GetDirectSignal,
}

// ---------------------------------------------------------------------------
// The evaluation context
// ---------------------------------------------------------------------------

///
/// [`WireSignal::Suppressed`] is the state
/// `RedStoneWireBlock.getBlockSignal` (`:285-290`) puts the block in for the
/// length of one `getBestNeighborSignal`, and nothing else in 26.1 clears it.
/// All three of the wire's redstone members read it, so a suppressed wire is
/// not a signal source, emits nothing on any side and directs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireSignal {
    /// `shouldSignal == true`, the field's value everywhere outside
    /// `getBlockSignal`, and the one it is restored to at `:288`.
    Emitting,
    /// `shouldSignal == false`, i.e. inside `getBlockSignal`.
    Suppressed,
}

/// Everything a `SignalGetter` question reads that is not its own arguments:
/// the `BlockGetter` it walks and the [`WireSignal`] the wire block is in.
///
/// Vanilla needs no such thing, because the reader is `this` and the flag is a
/// field. Both are ambient there and both are explicit here, for the reason the
/// module documentation gives: the flag decides whether a wire answers its own
/// power or zero, and a port that lost track of it would carry a wire's signal
/// into the very computation that is supposed to exclude it.
pub struct SignalContext<'a> {
    block_state_at: BlockStateAt<'a>,
    sources: SignalSources<'a>,
    wire_signal: WireSignal,
}

/// Live world reads beyond block states required for redstone signal evaluation.
pub trait SignalSourceReader {
    fn comparator_output(&self, _pos: BlockPos) -> i32 {
        0
    }

    fn chest_open_count(&self, _pos: BlockPos) -> i32 {
        0
    }

    fn jukebox_playing(&self, _pos: BlockPos) -> bool {
        false
    }

    fn shulker_open(&self, _pos: BlockPos) -> bool {
        false
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        bbb_protocol::content_registry::ContentRegistry::vanilla()
    }

    fn is_redstone_conductor(&self, state: i32) -> bool {
        is_redstone_conductor_with_registry(state, self.content_registry())
    }

    fn support_face(
        &self,
        state: i32,
        _pos: BlockPos,
        direction: Direction,
        support_type: SupportType,
    ) -> bool {
        bbb_server_types::block_queries::face_sturdy_with_registry(
            state,
            direction,
            support_type,
            self.content_registry(),
        ) == Some(true)
    }
}

/// Default state-only signal source reader with no live block-entity state.
#[derive(Clone, Copy, Default)]
pub struct EmptySignalSources;

impl SignalSourceReader for EmptySignalSources {}

/// State-only signal source reader using position-aware `face_sturdy_at`.
#[derive(Clone, Copy, Default)]
pub(crate) struct SturdySignalSources;

impl SignalSourceReader for SturdySignalSources {
    fn support_face(
        &self,
        state: i32,
        pos: BlockPos,
        direction: Direction,
        support_type: SupportType,
    ) -> bool {
        bbb_server_types::block_queries::face_sturdy_at(state, pos, direction, support_type)
    }
}

impl SignalSourceReader for LevelCtx<'_, '_> {
    fn comparator_output(&self, pos: BlockPos) -> i32 {
        self.block_entity_output_signal(pos)
    }

    fn chest_open_count(&self, pos: BlockPos) -> i32 {
        LevelCtx::chest_open_count(self, pos)
    }

    fn jukebox_playing(&self, pos: BlockPos) -> bool {
        LevelCtx::jukebox_playing(self, pos)
    }

    fn shulker_open(&self, pos: BlockPos) -> bool {
        self.shulker_box_open(pos)
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        LevelCtx::content_registry(self)
    }

    fn support_face(
        &self,
        state: i32,
        pos: BlockPos,
        direction: Direction,
        support_type: SupportType,
    ) -> bool {
        bbb_server_types::block_queries::face_sturdy_at_with_registry(
            state,
            pos,
            direction,
            support_type,
            LevelCtx::content_registry(self),
        )
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Default)]
pub(crate) struct TestSignalSources<'a> {
    pub comparator_output_at: Option<&'a dyn Fn(BlockPos) -> i32>,
    pub chest_open_count_at: Option<&'a dyn Fn(BlockPos) -> i32>,
    pub jukebox_playing_at: Option<&'a dyn Fn(BlockPos) -> bool>,
    pub shulker_open_at: Option<&'a dyn Fn(BlockPos) -> bool>,
}

#[cfg(test)]
impl SignalSourceReader for TestSignalSources<'_> {
    fn comparator_output(&self, pos: BlockPos) -> i32 {
        self.comparator_output_at.map_or(0, |lookup| lookup(pos))
    }

    fn chest_open_count(&self, pos: BlockPos) -> i32 {
        self.chest_open_count_at.map_or(0, |lookup| lookup(pos))
    }

    fn jukebox_playing(&self, pos: BlockPos) -> bool {
        self.jukebox_playing_at.is_some_and(|lookup| lookup(pos))
    }

    fn shulker_open(&self, pos: BlockPos) -> bool {
        self.shulker_open_at.is_some_and(|lookup| lookup(pos))
    }
}

/// The live reads a signal question makes beyond block states. Placement and
/// the tick bind the same ones, so a block placed beside a powered comparator,
/// an open trapped chest or a playing jukebox sees that power.
#[derive(Clone, Copy)]
pub struct SignalSources<'a>(&'a dyn SignalSourceReader);

impl Default for SignalSources<'_> {
    fn default() -> Self {
        Self(&EmptySignalSources)
    }
}

impl<'a> SignalSources<'a> {
    #[must_use]
    pub const fn new(reader: &'a dyn SignalSourceReader) -> Self {
        Self(reader)
    }

    /// `SignalContext::emitting` bound to these sources.
    pub fn emitting<'r>(self, block_state_at: BlockStateAt<'r>) -> SignalContext<'r>
    where
        'a: 'r,
    {
        SignalContext::emitting(block_state_at).with_sources(self)
    }
}

impl<'a> SignalContext<'a> {
    /// A context with `shouldSignal` at its resting value, which is what every
    /// caller outside `RedStoneWireBlock.getBlockSignal` asks in.
    pub fn emitting(block_state_at: BlockStateAt<'a>) -> Self {
        Self {
            block_state_at,
            sources: SignalSources::default(),
            wire_signal: WireSignal::Emitting,
        }
    }

    #[must_use]
    pub fn with_sources(mut self, sources: SignalSources<'a>) -> Self {
        self.sources = sources;
        self
    }

    #[must_use]
    pub fn with_reader(mut self, reader: &'a dyn SignalSourceReader) -> Self {
        self.sources = SignalSources::new(reader);
        self
    }

    /// The context `RedStoneWireBlock.getBlockSignal` (`:286-287`) opens.
    pub fn suppressed(block_state_at: BlockStateAt<'a>) -> Self {
        Self {
            block_state_at,
            sources: SignalSources::default(),
            wire_signal: WireSignal::Suppressed,
        }
    }

    #[must_use]
    pub fn comparator_output(&self, pos: BlockPos) -> i32 {
        self.sources.0.comparator_output(pos)
    }

    pub(crate) fn support_face(
        &self,
        state: i32,
        pos: BlockPos,
        direction: Direction,
        support_type: SupportType,
    ) -> bool {
        self.sources
            .0
            .support_face(state, pos, direction, support_type)
    }

    #[must_use]
    pub fn chest_open_count(&self, pos: BlockPos) -> i32 {
        self.sources.0.chest_open_count(pos)
    }

    /// `isRedstoneConductor` at `pos`: a shulker box is `dynamicShape`, so its
    /// default `isCollisionShapeFullBlock` (`BlockBehaviour.java:1003`) fails
    /// once the lid has moved (`ShulkerBoxBlock.java:152-156`).
    #[must_use]
    pub fn is_conductor_at(&self, pos: BlockPos, state: i32) -> bool {
        self.sources.0.is_redstone_conductor(state)
            && !(bbb_server_types::block_queries::is_shulker_box(state)
                && self.sources.0.shulker_open(pos))
    }

    #[must_use]
    pub fn jukebox_playing(&self, pos: BlockPos) -> bool {
        self.sources.0.jukebox_playing(pos)
    }

    /// `BlockGetter.getBlockState(pos)`.
    pub fn block_state(&mut self, pos: BlockPos) -> i32 {
        (self.block_state_at)(pos)
    }

    /// The `shouldSignal` this question is running under.
    #[must_use]
    pub const fn wire_signal(&self) -> WireSignal {
        self.wire_signal
    }

    /// The bare reader, for the bodies that take a [`BlockStateAt`] because
    /// they are not redstone questions at all -- `canSurvive` and the
    /// connection walk behind it.
    pub fn reader(&mut self) -> BlockStateAt<'_> {
        &mut *self.block_state_at
    }
}

// ---------------------------------------------------------------------------
// The two per-state questions
// ---------------------------------------------------------------------------

/// The class the generated catalog says owns one of the three redstone members
/// for a state, or `None` when the block declares no override at all and
/// inherits `BlockBehaviour`'s default.
fn signal_body_owner(
    block_state_id: i32,
    member: SignalMember,
) -> Option<VanillaBlockRedstoneSignalOverride> {
    let entry = vanilla_block_type(block_type_of(block_state_id)?)?;
    match member {
        SignalMember::IsSignalSource => entry.is_signal_source_override,
        SignalMember::GetSignal => entry.signal_override,
        SignalMember::GetDirectSignal => entry.direct_signal_override,
    }
}

pub fn is_signal_source(block_state_id: i32, wire_signal: WireSignal) -> bool {
    match signal_body_owner(block_state_id, SignalMember::IsSignalSource) {
        None => false,
        Some(VanillaBlockRedstoneSignalOverride::PoweredBlock) => true,
        Some(VanillaBlockRedstoneSignalOverride::RedstoneTorchBlock) => true,
        Some(VanillaBlockRedstoneSignalOverride::DiodeBlock) => true,
        Some(VanillaBlockRedstoneSignalOverride::TrappedChestBlock) => true,
        Some(VanillaBlockRedstoneSignalOverride::LecternBlock) => true,
        Some(VanillaBlockRedstoneSignalOverride::DetectorRailBlock) => true,
        Some(VanillaBlockRedstoneSignalOverride::JukeboxBlock) => true,
        Some(
            VanillaBlockRedstoneSignalOverride::ObserverBlock
            | VanillaBlockRedstoneSignalOverride::LightningRodBlock,
        ) => true,
        Some(
            VanillaBlockRedstoneSignalOverride::TargetBlock
            | VanillaBlockRedstoneSignalOverride::SculkSensorBlock,
        ) => true,
        Some(
            VanillaBlockRedstoneSignalOverride::DaylightDetectorBlock
            | VanillaBlockRedstoneSignalOverride::TripWireHookBlock,
        ) => true,
        Some(
            VanillaBlockRedstoneSignalOverride::LeverBlock
            | VanillaBlockRedstoneSignalOverride::ButtonBlock
            | VanillaBlockRedstoneSignalOverride::BasePressurePlateBlock,
        ) => true,
        Some(VanillaBlockRedstoneSignalOverride::RedStoneWireBlock) => {
            redstone_wire::is_signal_source(wire_signal)
        }
        Some(_) => false,
    }
}

pub fn signal(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    context: &mut SignalContext<'_>,
) -> i32 {
    match signal_body_owner(block_state_id, SignalMember::GetSignal) {
        None => 0,
        Some(VanillaBlockRedstoneSignalOverride::PoweredBlock) => SIGNAL_MAX,
        Some(
            VanillaBlockRedstoneSignalOverride::RedstoneTorchBlock
            | VanillaBlockRedstoneSignalOverride::RedstoneWallTorchBlock,
        ) => redstone_torch::signal(block_state_id, direction),
        Some(VanillaBlockRedstoneSignalOverride::DiodeBlock) => {
            if crate::block_behavior::comparator::is_comparator(block_state_id) {
                crate::block_behavior::comparator::signal(
                    block_state_id,
                    direction,
                    context.comparator_output(pos),
                )
            } else {
                diode::signal(block_state_id, direction).unwrap_or(0)
            }
        }
        Some(VanillaBlockRedstoneSignalOverride::RedStoneWireBlock) => {
            redstone_wire::signal(block_state_id, pos, direction, context)
        }
        Some(VanillaBlockRedstoneSignalOverride::TrappedChestBlock) => {
            crate::block_behavior::chest::trapped_signal(context.chest_open_count(pos))
        }
        Some(VanillaBlockRedstoneSignalOverride::LecternBlock) => {
            crate::block_behavior::lectern::signal(block_state_id)
        }
        Some(VanillaBlockRedstoneSignalOverride::JukeboxBlock) => {
            crate::block_behavior::jukebox::signal(context.jukebox_playing(pos))
        }
        Some(VanillaBlockRedstoneSignalOverride::DetectorRailBlock) => {
            crate::block_behavior::rail::detector_signal(block_state_id)
        }
        Some(
            VanillaBlockRedstoneSignalOverride::LeverBlock
            | VanillaBlockRedstoneSignalOverride::ButtonBlock,
        ) => crate::block_behavior::footing::face_attached_signal(block_state_id),
        Some(VanillaBlockRedstoneSignalOverride::BasePressurePlateBlock) => {
            crate::block_behavior::pressure_plate::signal(block_state_id)
        }
        Some(VanillaBlockRedstoneSignalOverride::ObserverBlock) => {
            crate::block_behavior::observer::signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::LightningRodBlock) => {
            crate::block_behavior::lightning_rod::signal(block_state_id)
        }
        Some(VanillaBlockRedstoneSignalOverride::TargetBlock) => {
            crate::block_behavior::target::signal(block_state_id)
        }
        Some(VanillaBlockRedstoneSignalOverride::DaylightDetectorBlock) => {
            crate::block_behavior::daylight_detector::signal(block_state_id)
        }
        Some(VanillaBlockRedstoneSignalOverride::TripWireHookBlock) => {
            crate::block_behavior::tripwire_hook::signal(block_state_id)
        }
        Some(
            VanillaBlockRedstoneSignalOverride::SculkSensorBlock
            | VanillaBlockRedstoneSignalOverride::CalibratedSculkSensorBlock,
        ) => crate::block_behavior::sculk_sensor::signal(block_state_id, direction),
        Some(_) => 0,
    }
}

pub fn direct_signal(
    block_state_id: i32,
    pos: BlockPos,
    direction: Direction,
    context: &mut SignalContext<'_>,
) -> i32 {
    match signal_body_owner(block_state_id, SignalMember::GetDirectSignal) {
        None => 0,
        Some(VanillaBlockRedstoneSignalOverride::RedstoneTorchBlock) => {
            redstone_torch::direct_signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::DiodeBlock) => {
            if crate::block_behavior::comparator::is_comparator(block_state_id) {
                crate::block_behavior::comparator::signal(
                    block_state_id,
                    direction,
                    context.comparator_output(pos),
                )
            } else {
                diode::direct_signal(block_state_id, direction).unwrap_or(0)
            }
        }
        Some(VanillaBlockRedstoneSignalOverride::RedStoneWireBlock) => {
            redstone_wire::direct_signal(block_state_id, pos, direction, context)
        }
        Some(VanillaBlockRedstoneSignalOverride::TrappedChestBlock) => {
            crate::block_behavior::chest::trapped_direct_signal(
                direction,
                context.chest_open_count(pos),
            )
        }
        Some(VanillaBlockRedstoneSignalOverride::LecternBlock) => {
            crate::block_behavior::lectern::direct_signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::DetectorRailBlock) => {
            crate::block_behavior::rail::detector_direct_signal(block_state_id, direction)
        }
        Some(
            VanillaBlockRedstoneSignalOverride::LeverBlock
            | VanillaBlockRedstoneSignalOverride::ButtonBlock,
        ) => crate::block_behavior::footing::face_attached_direct_signal(block_state_id, direction),
        Some(VanillaBlockRedstoneSignalOverride::BasePressurePlateBlock) => {
            crate::block_behavior::pressure_plate::direct_signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::ObserverBlock) => {
            crate::block_behavior::observer::signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::LightningRodBlock) => {
            crate::block_behavior::lightning_rod::direct_signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::TripWireHookBlock) => {
            crate::block_behavior::tripwire_hook::direct_signal(block_state_id, direction)
        }
        Some(VanillaBlockRedstoneSignalOverride::SculkSensorBlock) => {
            crate::block_behavior::sculk_sensor::direct_signal(block_state_id, direction)
        }
        Some(_) => 0,
    }
}

///
/// See this module's own documentation for why the bit and not a shape test.
/// An id the generated 26.1 registry does not know answers `false`, which is
/// air's answer and the one that folds nothing in. The bit samples shulker
/// boxes closed; [`SignalContext::is_conductor_at`] corrects an open one.
#[must_use]
pub fn is_redstone_conductor(block_state_id: i32) -> bool {
    is_redstone_conductor_with_registry(
        block_state_id,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

#[must_use]
pub fn is_redstone_conductor_with_registry(
    block_state_id: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bool {
    if let Some(facts) = vanilla_block_state_support_facts(block_state_id) {
        return facts.redstone_conductor();
    }
    content_registry
        .block_by_state_id(block_state_id)
        .is_some_and(|(block, _)| block.full_cube_collision)
}

#[must_use]
pub fn has_analog_output_signal(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .is_some_and(|entry| entry.analog_output_override.is_some())
}

/// State-only `getAnalogOutputSignal` bodies selected by the generated owner
/// column. `None` leaves block-entity and entity-backed owners to
/// `simulation_level::analog_output`.
#[must_use]
pub fn state_only_analog_output(block_state_id: i32) -> Option<i32> {
    let owner = block_type_of(block_state_id)
        .and_then(vanilla_block_type)?
        .analog_output_override?;
    match owner {
        VanillaBlockAnalogOutputOverride::BlockBehaviour => Some(0),
        VanillaBlockAnalogOutputOverride::LayeredCauldronBlock => {
            int_property(block_state_id, "level")
        }
        VanillaBlockAnalogOutputOverride::LavaCauldronBlock => Some(3),
        VanillaBlockAnalogOutputOverride::CandleCakeBlock => Some(14),
        VanillaBlockAnalogOutputOverride::EndPortalFrameBlock => {
            bool_property(block_state_id, "eye").map(|eye| if eye { SIGNAL_MAX } else { 0 })
        }
        VanillaBlockAnalogOutputOverride::RespawnAnchorBlock => {
            int_property(block_state_id, "charges")
                .map(|charge| ((charge as f32 / 4.0) * SIGNAL_MAX as f32).floor() as i32)
        }
        VanillaBlockAnalogOutputOverride::CakeBlock => Some(cake::analog_output(block_state_id)),
        VanillaBlockAnalogOutputOverride::ComposterBlock => {
            Some(composter::analog_output(block_state_id))
        }
        VanillaBlockAnalogOutputOverride::CopperBulbBlock => {
            Some(copper_bulb::analog_output(block_state_id))
        }
        VanillaBlockAnalogOutputOverride::BeehiveBlock => {
            Some(beehive::analog_output(block_state_id))
        }
        VanillaBlockAnalogOutputOverride::CopperGolemStatueBlock => {
            Some(copper_golem_statue::analog_output(block_state_id))
        }
        VanillaBlockAnalogOutputOverride::CreakingHeartBlock => {
            (string_property(block_state_id, "creaking_heart_state").as_deref() == Some("uprooted"))
                .then_some(0)
        }
        VanillaBlockAnalogOutputOverride::DetectorRailBlock => {
            (bool_property(block_state_id, "powered") == Some(false)).then_some(0)
        }
        VanillaBlockAnalogOutputOverride::LecternBlock
        | VanillaBlockAnalogOutputOverride::ChiseledBookShelfBlock
        | VanillaBlockAnalogOutputOverride::ShelfBlock
        | VanillaBlockAnalogOutputOverride::CrafterBlock
        | VanillaBlockAnalogOutputOverride::CommandBlock
        | VanillaBlockAnalogOutputOverride::JukeboxBlock
        | VanillaBlockAnalogOutputOverride::SculkSensorBlock
        | VanillaBlockAnalogOutputOverride::HopperBlock
        | VanillaBlockAnalogOutputOverride::BarrelBlock
        | VanillaBlockAnalogOutputOverride::DispenserBlock
        | VanillaBlockAnalogOutputOverride::ShulkerBoxBlock
        | VanillaBlockAnalogOutputOverride::DecoratedPotBlock
        | VanillaBlockAnalogOutputOverride::ChestBlock
        | VanillaBlockAnalogOutputOverride::AbstractFurnaceBlock
        | VanillaBlockAnalogOutputOverride::BrewingStandBlock => None,
    }
}

#[must_use]
pub fn is_diode(block_state_id: i32) -> bool {
    signal_body_owner(block_state_id, SignalMember::IsSignalSource)
        == Some(VanillaBlockRedstoneSignalOverride::DiodeBlock)
}

// ---------------------------------------------------------------------------
// `SignalGetter`
// ---------------------------------------------------------------------------

pub fn get_direct_signal(
    pos: BlockPos,
    direction: Direction,
    context: &mut SignalContext<'_>,
) -> i32 {
    let state = context.block_state(pos);
    direct_signal(state, pos, direction, context)
}

///
/// Transcribed unrolled, as vanilla writes it, because the early returns are
/// what make the walk order observable — the sides are visited in
/// [`DIRECTIONS`] order and a side answering [`SIGNAL_MAX`] stops the rest
/// from being read at all.
///
/// The last line (`:45`) is `return result >= 15 ? result : result;` — both
/// arms are the same expression, so it is an identity and is transcribed as
/// the bare `result` it evaluates to. It is not a bug to fix into something
/// else; the sixth side simply has no early return left to take.
pub fn get_direct_signal_to(pos: BlockPos, context: &mut SignalContext<'_>) -> i32 {
    // `:18`.
    let mut result = 0;
    // `:19-22`.
    result = result.max(get_direct_signal(
        relative(pos, Direction::Down),
        Direction::Down,
        context,
    ));
    if result >= SIGNAL_MAX {
        return result;
    }
    // `:24-27`.
    result = result.max(get_direct_signal(
        relative(pos, Direction::Up),
        Direction::Up,
        context,
    ));
    if result >= SIGNAL_MAX {
        return result;
    }
    // `:29-32`.
    result = result.max(get_direct_signal(
        relative(pos, Direction::North),
        Direction::North,
        context,
    ));
    if result >= SIGNAL_MAX {
        return result;
    }
    // `:34-37`.
    result = result.max(get_direct_signal(
        relative(pos, Direction::South),
        Direction::South,
        context,
    ));
    if result >= SIGNAL_MAX {
        return result;
    }
    // `:39-42`.
    result = result.max(get_direct_signal(
        relative(pos, Direction::West),
        Direction::West,
        context,
    ));
    if result >= SIGNAL_MAX {
        return result;
    }
    // `:44`.
    result = result.max(get_direct_signal(
        relative(pos, Direction::East),
        Direction::East,
        context,
    ));
    // `:45`, the identity ternary.
    result
}

///
/// Nothing in this slice calls it — its callers are `DiodeBlock` and
/// `ComparatorBlock`, neither of which has a body here — but the three special
/// cases it hard-codes are the whole reason it exists and are derived rather
/// than deferred: a redstone block is a flat 15 (`:52-53`) even though its own
/// `getDirectSignal` is 0, and redstone wire answers its `POWER` property
/// (`:54-55`) rather than either signal member.
pub fn get_control_input_signal(
    pos: BlockPos,
    direction: Direction,
    only_diodes: bool,
    context: &mut SignalContext<'_>,
) -> i32 {
    // `:49`.
    let state = context.block_state(pos);
    if only_diodes {
        // `:51`.
        return if is_diode(state) {
            get_direct_signal(pos, direction, context)
        } else {
            0
        };
    }
    // `:52-53`.
    if block_type_of(state) == Some(VANILLA_BLOCK_REDSTONE_BLOCK_ID) {
        return SIGNAL_MAX;
    }
    // `:54-55`. Every `redstone_wire` state carries `POWER`, so the fallback
    // is unreachable; it is there because the property read is fallible in
    // this port and vanilla's `getValue` is not. Note that this arm reads the
    // property straight off the state and never asks `getSignal`, so a diode
    // beside a wire is the one reader `shouldSignal` cannot silence.
    if block_type_of(state) == Some(VANILLA_BLOCK_REDSTONE_WIRE_ID) {
        return int_property(state, POWER).unwrap_or(0);
    }
    // `:57`.
    if is_signal_source(state, context.wire_signal()) {
        get_direct_signal(pos, direction, context)
    } else {
        0
    }
}

pub fn has_signal(pos: BlockPos, direction: Direction, context: &mut SignalContext<'_>) -> bool {
    get_signal(pos, direction, context) > 0
}

///
/// The `max` with [`get_direct_signal_to`] happens **only** when the state at
/// `pos` is a redstone conductor (`:68`). That fold is the "powered block"
/// rule: a lever on a stone block powers the stone, and the same lever on a
/// redstone block powers nothing extra, because
/// [`is_redstone_conductor`] answers false there.
pub fn get_signal(pos: BlockPos, direction: Direction, context: &mut SignalContext<'_>) -> i32 {
    // `:66`.
    let state = context.block_state(pos);
    // `:67`.
    let own = signal(state, pos, direction, context);
    // `:68`.
    if context.is_conductor_at(pos, state) {
        own.max(get_direct_signal_to(pos, context))
    } else {
        own
    }
}

///
/// Transcribed unrolled, as vanilla writes it: the six sides are visited in
/// [`DIRECTIONS`] order and the first one answering a positive signal stops
/// the walk, so a block below a source is read and a block above it is not.
pub fn has_neighbor_signal(pos: BlockPos, context: &mut SignalContext<'_>) -> bool {
    // `:72-73`.
    if get_signal(relative(pos, Direction::Down), Direction::Down, context) > 0 {
        return true;
    }
    // `:74-75`.
    if get_signal(relative(pos, Direction::Up), Direction::Up, context) > 0 {
        return true;
    }
    // `:76-77`.
    if get_signal(relative(pos, Direction::North), Direction::North, context) > 0 {
        return true;
    }
    // `:78-79`.
    if get_signal(relative(pos, Direction::South), Direction::South, context) > 0 {
        return true;
    }
    // `:81`, the one line that carries the last two sides.
    if get_signal(relative(pos, Direction::West), Direction::West, context) > 0 {
        return true;
    }
    get_signal(relative(pos, Direction::East), Direction::East, context) > 0
}

///
/// This one vanilla *does* write as a loop over [`DIRECTIONS`], and the early
/// return at [`SIGNAL_MAX`] (`:90-92`) means a side already at 15 leaves the
/// remaining sides unread.
pub fn get_best_neighbor_signal(pos: BlockPos, context: &mut SignalContext<'_>) -> i32 {
    // `:86`.
    let mut best = 0;
    // `:88-97`.
    for direction in DIRECTIONS {
        let side = get_signal(relative(pos, direction), direction, context);
        if side >= SIGNAL_MAX {
            return SIGNAL_MAX;
        }
        if side > best {
            best = side;
        }
    }
    // `:99`.
    best
}

#[cfg(test)]
mod tests;
