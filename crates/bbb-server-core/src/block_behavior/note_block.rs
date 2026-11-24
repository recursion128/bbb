//! `NoteBlock` (`net/minecraft/world/level/block/NoteBlock.java`).
//!
//! The block-event template for this slice, and the first block in this crate
//! that *produces* one. [`crate::block_events`] already carries the queue, the
//! de-duplication and the drain loop; until now nothing called
//! `Level.blockEvent` (`ServerLevel.java`), so the whole mechanism
//! was dead. A note block is the one 26.1 producer in
//! `net/minecraft/world/level/block/` that needs no block entity: pistons,
//! chests, bells and spawners all read one from `triggerEvent`.
//!
//! # The two producers, and why only one is here
//!
//! `NoteBlock` enqueues from exactly two places, both through `playNote`
//! (`NoteBlock.java`):
//!
//! * `useWithoutItem` (`:124-133`) — a right click, which is what this server
//!   wires up.
//! * `neighborChanged` (`:88-99`) — a redstone edge, read off
//!   `level.hasNeighborSignal(pos)` (`:91`). It enqueues only on the rising
//!   edge and stores both edges in `BlockStateProperties.POWERED`.
//!
//! `attack` (`:136-141`) is a third caller of `playNote`, reached from the left
//! click that starts a break. It is not wired either: this crate's break path
//! has no `BlockState.attack` leg, and adding one is a separate seam from the
//! use path the differential drives.
//!
//! # What `triggerEvent` owns, and what it does not
//!
//! `NoteBlock.triggerEvent` (`:148-175`) does three things: it adds a particle,
//! it plays a sound, and it answers a boolean. Two of the three are this
//! module's. The boolean is the one that matters — `ServerLevel.doBlockEvent`
//! (`ServerLevel.java`) turns a `true` into the
//! `ClientboundBlockEventPacket` broadcast at `ServerLevel.java` —
//! and the sound rides out with it on [`BlockTriggerOutcome`].
//!
//! * The particle (`NoteBlock.java`) is `Level.addParticle`, whose body is
//!   empty (`Level.java`) and which `ServerLevel` does not override.
//!   The note particle is the receiving client's own work, off the very packet
//!   this event broadcasts.
//! * The sound (`:171-173`) is **not** client-side: `ServerLevel.playSeededSound`
//!   (`ServerLevel.java`) broadcasts a `ClientboundSoundPacket` of
//!   its own, so it *is* this module's — it is carried out of
//!   [`trigger_event`] on [`BlockTriggerOutcome`] and broadcast by the drain
//!   loop that owns the player list. The instrument-to-`SoundEvents` join it
//!   needs (`NoteBlockInstrument.java`) is the generated
//!   [`bbb_protocol::note_block_instruments`] catalog.
//! * `player.awardStat(Stats.TUNE_NOTEBLOCK)` (`:129`, `Stats.java`) needs
//!   the per-player stat counters, which live with the player, not the block.
//!
//! One `triggerEvent` branch *is* modelled, because it decides the boolean: a
//! `CUSTOM_HEAD` instrument answers false outright when the block above is not
//! a skull carrying a note-block sound (`:160-164`, `:177-179`). The caller
//! reads that identifier off the live [`crate::block_entity::skull`] object;
//! `None` is vanilla's silent player head.

use super::{BlockShapeCtx, LevelCtx};
use std::collections::HashSet;
use std::sync::OnceLock;

use crate::random::GameRandom;
use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::block_state_types::{
    vanilla_block_note_instrument, vanilla_block_state_property,
};
use bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::note_block_instruments::{
    vanilla_note_block_instrument_for_name, NoteBlockInstrumentCatalogEntry,
    NoteBlockInstrumentType,
};
use bbb_protocol::packets::{
    BlockPos, Direction, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::PersistentRegistryTagMembers;

use crate::fluid::{above, below, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::fluid::block_state_is_air;

use super::{
    block_type_of, bool_property, default_block_state_id, int_property, with_property,
    BlockInteractionPlayer, BlockTriggerOutcome, BlockUseResult,
};

/// `NoteBlock.INSTRUMENT` (`NoteBlock.java:39`), i.e.
/// `BlockStateProperties.NOTEBLOCK_INSTRUMENT`
/// (`BlockStateProperties.java:133`), whose serialised name is `instrument`.
const INSTRUMENT: &str = "instrument";

/// `NoteBlock.NOTE` (`NoteBlock.java:41`), i.e. `BlockStateProperties.NOTE`
/// (`BlockStateProperties.java:118`).
const NOTE: &str = "note";

/// The bounds of `BlockStateProperties.NOTE`
/// (`BlockStateProperties.java:118`, `IntegerProperty.create("note", 0, 24)`).
///
/// Both halves are needed by [`cycle_note`]: `IntegerProperty` builds its
/// possible values as `IntStream.range(min, max + 1)`
/// (`IntegerProperty.java:25`), so they ascend from the minimum, and
/// `StateHolder.findNextInCollection` (`StateHolder.java:36-39`) wraps past the
/// last one back to the first.
const NOTE_MIN: i32 = 0;
const NOTE_MAX: i32 = 24;

/// `NoteBlockInstrument.HARP`'s serialised name (`NoteBlockInstrument.java:9`).
///
/// It is both the `BlockBehaviour.Properties` default every block starts at
/// (`BlockBehaviour.java:997`) and the instrument `setInstrument` falls back to
/// when the block below cannot supply one (`NoteBlock.java:61`).
const HARP: &str = "harp";

/// The volume `NoteBlock.triggerEvent` plays every note at
/// (`NoteBlock.java:172`).
const TRIGGER_VOLUME: f32 = 3.0;

/// The `NoteBlockInstrument` constant one serialised name refers to
/// (`NoteBlockInstrument.java`).
///
/// The three facts a constant carries — the serialised name, the `SoundEvent`
/// and the `Type` — are one generated catalog,
/// [`bbb_protocol::note_block_instruments`], which this module and the client's
/// own note-block code both read. Nothing here restates the constant list.
pub(super) fn instrument_type(instrument: &str) -> Option<NoteBlockInstrumentType> {
    vanilla_note_block_instrument_for_name(instrument).map(|entry| entry.instrument_type)
}

/// `NoteBlockInstrument.isTunable` (`NoteBlockInstrument.java`).
fn is_tunable(instrument: &NoteBlockInstrumentCatalogEntry) -> bool {
    instrument.instrument_type == NoteBlockInstrumentType::BaseBlock
}

/// `NoteBlockInstrument.hasCustomSound` (`NoteBlockInstrument.java`).
fn has_custom_sound(instrument: &NoteBlockInstrumentCatalogEntry) -> bool {
    instrument.instrument_type == NoteBlockInstrumentType::Custom
}

/// `NoteBlockInstrument.worksAboveNoteBlock` (`NoteBlockInstrument.java`).
///
/// An unclassifiable name answers as `BASE_BLOCK` does, which is
/// `Properties.instrument`'s own default (`BlockBehaviour.java`) and the
/// closed side of this predicate.
fn works_above_note_block(instrument: &str) -> bool {
    instrument_type(instrument).is_some_and(|kind| kind != NoteBlockInstrumentType::BaseBlock)
}

/// `BlockState.instrument()` (`BlockBehaviour.java`), which reads the
/// per-block `Properties.instrument` (`BlockBehaviour.java`) rather than
/// anything on the state.
///
/// A block the generated catalog does not know answers [`HARP`], which is what
/// `Properties` starts every block at (`BlockBehaviour.java`).
fn block_instrument(block_state_id: i32) -> &'static str {
    block_type_of(block_state_id)
        .and_then(vanilla_block_note_instrument)
        .unwrap_or(HARP)
}

/// `state.getValue(NoteBlock.INSTRUMENT)`: the note block's *own* stored
/// instrument, which is a state property and not
/// `BlockBehaviour.Properties.instrument`.
///
/// [`block_instrument`] is the other one — `BlockState.instrument()`
/// (`BlockBehaviour.java`) — and the two are different facts about
/// different blocks. `playNote` (`NoteBlock.java`) and `triggerEvent`
/// (`:149`) both read the stored property off the note block itself; only
/// `setInstrument` (`:55`, `:60`) reads the neighbours' block-level one.
fn stored_instrument(block_state_id: i32) -> &'static str {
    vanilla_block_state_property(block_state_id, INSTRUMENT).unwrap_or(HARP)
}

/// `NoteBlock.setInstrument` (`NoteBlock.java`).
///
/// The block *above* wins whenever its instrument works above a note block —
/// that is the mob-head stack — and only otherwise does the block below decide.
/// The below leg has a twist worth spelling out: an instrument that works above
/// a note block is *rejected* there and replaced by [`HARP`] (`:61`), so a skull
/// under a note block gives a harp, not its own voice.
///
/// Written as a pure function of three states because that is what it is: both
/// callers, `getStateForPlacement` (`:66-68`) and `updateShape` (`:71-85`),
/// differ only in which state they hand in.
#[must_use]
pub fn set_instrument(block_state_id: i32, above_state: i32, below_state: i32) -> i32 {
    let instrument_above = block_instrument(above_state);
    if works_above_note_block(instrument_above) {
        return with_property(block_state_id, INSTRUMENT, instrument_above)
            .unwrap_or(block_state_id);
    }
    let instrument_below = block_instrument(below_state);
    let new_below = if works_above_note_block(instrument_below) {
        HARP
    } else {
        instrument_below
    };
    with_property(block_state_id, INSTRUMENT, new_below).unwrap_or(block_state_id)
}

/// `NoteBlock.getStateForPlacement` (`NoteBlock.java`): the default state
/// (`registerDefaultState`, `:51`) with [`set_instrument`] applied at the
/// position being placed into.
///
/// `BlockPlaceContext.getClickedPos` is the position the block goes into, so
/// the two neighbour states are read there and not at the face that was hit.
#[must_use]
pub fn state_for_placement(above_state: i32, below_state: i32) -> i32 {
    set_instrument(
        default_block_state_id(VANILLA_BLOCK_NOTE_BLOCK_ID),
        above_state,
        below_state,
    )
}

/// `NoteBlock.updateShape` (`NoteBlock.java`).
///
/// Only a vertical neighbour change re-reads the instrument (`:81`,
/// `directionToNeighbour.getAxis() == Direction.Axis.Y`, whose two members are
/// `DOWN` and `UP`, `Direction.java`); every other side falls through to
/// `BlockBehaviour.updateShape` (`BlockBehaviour.java`), which returns
/// the state unchanged.
pub(super) fn update_shape(
    level: &BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
) -> i32 {
    if !matches!(direction_to_neighbour, Direction::Down | Direction::Up) {
        return block_state_id;
    }
    set_instrument(
        block_state_id,
        level.block_state(above(pos)),
        level.block_state(below(pos)),
    )
}

/// `state.cycle(NoteBlock.NOTE)` (`NoteBlock.java`).
///
/// `StateHolder.cycle` (`StateHolder.java`) takes the next value in
/// `Property.getPossibleValues()` and wraps past the end
/// (`StateHolder.findNextInCollection`, `StateHolder.java`); for an
/// `IntegerProperty` those values ascend from the minimum
/// (`IntegerProperty.java`), so the cycle is `note + 1` around the range.
fn cycle_note(block_state_id: i32) -> i32 {
    let Some(note) = int_property(block_state_id, NOTE) else {
        return block_state_id;
    };
    let next = if note >= NOTE_MAX { NOTE_MIN } else { note + 1 };
    with_property(block_state_id, NOTE, &next.to_string()).unwrap_or(block_state_id)
}

/// `NoteBlock.playNote` (`NoteBlock.java`).
///
/// The guard (`:102`) is the whole reason a note block can be silenced by
/// covering it: a tunable instrument only sounds with air directly above, while
/// a mob-head instrument — which by definition came *from* the block above —
/// always sounds. `level.blockEvent(pos, this, 0, 0)` (`:103`) is the enqueue,
/// and both parameters are literal zeros: `NoteBlock.triggerEvent` reads
/// neither of them (`:148-175`), taking the note and the instrument off the
/// state instead.
fn play_note(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    source_entity_id: Option<i32>,
) {
    if works_above_note_block(stored_instrument(block_state_id))
        || block_state_is_air(level.block_state(above(pos)))
    {
        level.block_event(pos, VANILLA_BLOCK_NOTE_BLOCK_ID, 0, 0);
        level.game_event_from_entity(
            VanillaGameEvent::NOTE_BLOCK_PLAY,
            pos,
            source_entity_id,
            None,
        );
    }
}

/// `NoteBlock.attack` (`NoteBlock.java`) without its stat award.
pub(super) fn attack(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) {
    play_note(level, pos, block_state_id, player.source_entity_id());
}

/// `NoteBlock.neighborChanged` (`NoteBlock.java`).
pub(super) fn neighbor_changed(level: &mut LevelCtx<'_, '_>, pos: BlockPos, block_state_id: i32) {
    let signal = level.has_neighbor_signal(pos);
    let Some(powered) = bool_property(block_state_id, "powered") else {
        return;
    };
    if signal == powered {
        return;
    }
    if signal {
        play_note(level, pos, block_state_id, None);
    }
    let Some(updated) = with_property(
        block_state_id,
        "powered",
        if signal { "true" } else { "false" },
    ) else {
        return;
    };
    level.set_block(
        pos,
        updated,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
}

/// `ItemTags.NOTE_BLOCK_TOP_INSTRUMENTS` (`ItemTags.java`).
///
/// Note the constant's Java name and its tag id disagree: the field is
/// `NOTE_BLOCK_TOP_INSTRUMENTS` but it binds `noteblock_top_instruments`,
/// which is the id the data file and the generated tag snapshot carry. Its
/// seven members are the six mob heads plus `player_head`, i.e. exactly the
/// items whose block has a `worksAboveNoteBlock` instrument.
fn note_block_top_instruments() -> &'static HashSet<i32> {
    static IDS: OnceLock<HashSet<i32>> = OnceLock::new();
    IDS.get_or_init(|| {
        let PersistentRegistryTagMembers::Present(members) =
            vanilla_persistent_registry_tag_members_26_1(
                "minecraft:item",
                "minecraft:noteblock_top_instruments",
            )
        else {
            return HashSet::new();
        };
        members
            .iter()
            .filter_map(|member| vanilla_item_type_for_resource_id(member))
            .map(|entry| entry.id)
            .collect()
    })
}

/// `NoteBlock.useItemOn` (`NoteBlock.java`).
///
/// One test with two halves (`:118-120`): a head that can sit on top of a note
/// block, clicked on the note block's *upward* face, passes — which is what
/// lets the head be placed there instead of the note being tuned. Anything else
/// falls through to `BlockBehaviour.useItemOn`
/// (`BlockBehaviour.java`), the request to try the empty hand.
///
/// Placing that head is not this crate's: it needs `SkullBlock`'s own
/// placement and a block entity. The gate is still transcribed, because it is
/// what decides whether the *note* moves.
pub(super) fn use_item_on(held_item_type_id: Option<i32>, face: Direction) -> BlockUseResult {
    let head_on_top = held_item_type_id
        .is_some_and(|item_type_id| note_block_top_instruments().contains(&item_type_id));
    if head_on_top && face == Direction::Up {
        BlockUseResult::Pass
    } else {
        BlockUseResult::TryWithEmptyHand
    }
}

/// `NoteBlock.useWithoutItem` (`NoteBlock.java`).
///
/// Three statements under the server-side gate (`:125`), in vanilla's order:
/// the note advances, the new state is written with `Block.UPDATE_ALL` (`:127`,
/// flag 3), and only then does `playNote` see it — so the note the block event
/// ends up playing is the *new* one, never the one that was standing there. The
/// stat award (`:130`) has no owner here.
///
/// Returns `InteractionResult.SUCCESS.consumesAction()` (`:132`,
/// `InteractionResult.java`), which is unconditionally true: a note block
/// consumes the click even when its instrument is muffled and no event is
/// queued.
pub(crate) fn use_without_item(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    block_state_id: i32,
    player: BlockInteractionPlayer,
) -> bool {
    let cycled = cycle_note(block_state_id);
    level.set_block(
        pos,
        cycled,
        SET_BLOCK_UPDATE_ALL,
        crate::neighbor_updater::UPDATE_LIMIT,
    );
    play_note(level, pos, cycled, player.source_entity_id());
    true
}

/// `NoteBlock.getPitchFromNote` (`NoteBlock.java`).
///
/// The subtraction is integer and only the division promotes to `double`, so
/// the exponent is evaluated at `f64` width and narrowed exactly once, which is
/// what the `(float)` cast does.
fn pitch_from_note(note: i32) -> f32 {
    2.0_f64.powf(f64::from(note - 12) / 12.0) as f32
}

/// `NoteBlock.triggerEvent` (`NoteBlock.java`): the boolean
/// `ServerLevel.doBlockEvent` (`ServerLevel.java`) consumes, plus the
/// sound `level.playSeededSound` (`:171-173`) plays on the way to it.
///
///
/// `b0` and `b1` are not read — see [`play_note`] — and neither is the level,
/// except for the one branch that can answer false: a `CUSTOM_HEAD` instrument
/// looks for a `SkullBlockEntity` above carrying a note-block sound (`:160-164`,
/// `:177-179`) and gives up when there is none. The caller supplies that
/// identifier; `None` is vanilla's silent player head.
///
/// Every other instrument answers true, which is what puts the
/// `ClientboundBlockEventPacket` on the wire.
pub(super) fn trigger_event(
    pos: BlockPos,
    block_state_id: i32,
    random: &GameRandom,
    skull_note_block_sound_above: Option<&str>,
) -> BlockTriggerOutcome {
    // `:149`.
    let Some(instrument) =
        vanilla_note_block_instrument_for_name(stored_instrument(block_state_id))
    else {
        // Unreachable: every value the generated block-state palette can put in
        // a note block's `instrument` property is a name the generated
        // instrument catalog carries, which
        // `every_note_block_instrument_value_is_in_the_generated_catalog` pins.
        // Answering false is `BlockBehaviour.triggerEvent`'s own default
        // (`BlockBehaviour.java`).
        return BlockTriggerOutcome::not_triggered();
    };
    // `:151-157`. The particle `:154` adds is `Level.addParticle`, whose body
    // is empty (`Level.java`); only the pitch survives on a server.
    let pitch = if is_tunable(instrument) {
        let Some(note) = int_property(block_state_id, NOTE) else {
            // Unreachable for the same reason: `createBlockStateDefinition`
            // (`:182-184`) puts `NOTE` on every note-block state, so vanilla's
            // `getValue` at `:152` cannot fail.
            return BlockTriggerOutcome::not_triggered();
        };
        pitch_from_note(note)
    } else {
        // `:156`.
        1.0
    };
    if has_custom_sound(instrument) {
        // `:160-164` through `getCustomSoundId` (`:177-179`).
        let Some(sound_id) = skull_note_block_sound_above.filter(|id| !id.is_empty()) else {
            return BlockTriggerOutcome::not_triggered();
        };
        return BlockTriggerOutcome {
            triggered: true,
            sound: Some(SoundEvent {
                sound: SoundEventHolder::Direct {
                    location: sound_id.to_string(),
                    fixed_range: None,
                },
                source: SoundSource::Records,
                position: Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y) + 0.5,
                    z: f64::from(pos.z) + 0.5,
                },
                volume: TRIGGER_VOLUME,
                pitch,
                seed: random.next_long(),
            }),
        };
    }
    // `:168`, sent by registry reference the way `ByteBufCodecs.holderRegistry`
    // writes a `Holder` that is not inline.
    let Some(registry_id) = vanilla_sound_event_registry_id(instrument.sound_resource_id) else {
        // Two generated catalogs disagreeing is a test failure
        // (`every_instrument_sound_is_in_the_generated_sound_event_catalog`),
        // not a reason to swallow the block event: `:174` answers true whatever
        // was played, and the boolean is what a client observes.
        return BlockTriggerOutcome {
            triggered: true,
            sound: None,
        };
    };
    BlockTriggerOutcome {
        // `:174`.
        triggered: true,
        // `:171-173`.
        sound: Some(SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            // `SoundSource.RECORDS` (`:172`), which is what puts a note block
            // on the jukebox volume slider rather than the block one.
            source: SoundSource::Records,
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume: TRIGGER_VOLUME,
            pitch,
            // `level.getRandom().nextLong()` (`:172`), i.e. `Level.random`
            // (`Level.java`). The caller hands the level's own
            // generator in, so the draw lands in the same sequence every other
            // block body draws from.
            seed: random.next_long(),
        }),
    }
}
