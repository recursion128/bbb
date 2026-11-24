//! What a `SignalGetter` answers, pinned against `SignalGetter.java`,
//! `PoweredBlock.java`, `BlockBehaviour.java` and the two `Blocks.java`
//! registrations that decide who conducts.

use std::collections::HashMap;

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;

use crate::fluid::with_property;

use super::*;

// ---------------------------------------------------------------------------
// States and the recording world
// ---------------------------------------------------------------------------

fn default_state(name: &str) -> i32 {
    let entry = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"));
    vanilla_block_default_state_id(entry.id)
        .unwrap_or_else(|| panic!("{name} has no default state"))
}

/// One state by name and properties, for the two blocks whose *default* state
/// is not the interesting one.
fn state_of(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties: std::collections::BTreeMap<String, String> = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties)
        .unwrap_or_else(|| panic!("no built-in state for {name} {properties:?}"))
}

fn air() -> i32 {
    vanilla_block_state_id_by_name_and_properties("minecraft:air", &Default::default())
        .expect("air has one state")
}

const fn at(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

/// Deliberately not the origin, so a read at the wrong position cannot pass by
/// coincidence.
const ORIGIN: BlockPos = at(6, 71, -3);

/// A world that answers block states and remembers which positions were asked,
/// in order.
struct Probe {
    blocks: HashMap<(i32, i32, i32), i32>,
    reads: Vec<BlockPos>,
}

impl Probe {
    fn new(entries: &[(BlockPos, i32)]) -> Self {
        Self {
            blocks: entries
                .iter()
                .map(|(pos, state)| ((pos.x, pos.y, pos.z), *state))
                .collect(),
            reads: Vec::new(),
        }
    }

    fn read(&mut self, pos: BlockPos) -> i32 {
        self.reads.push(pos);
        self.blocks
            .get(&(pos.x, pos.y, pos.z))
            .copied()
            .unwrap_or_else(air)
    }
}

// ---------------------------------------------------------------------------
// `isRedstoneConductor` is the generated bit, not the collision box
// ---------------------------------------------------------------------------

#[test]
fn the_redstone_block_does_not_conduct_and_soul_sand_does() {
    // The two registrations that go against the collision box, which is what
    // makes this the proof that the generated per-state bit is being read and
    // not `isCollisionShapeFullBlock` reconstructed from a shape.
    //
    // `REDSTONE_BLOCK` is `isRedstoneConductor(Blocks::never)`
    // (`Blocks.java:2889`) — a full cube that does not conduct.
    assert!(!is_redstone_conductor(default_state(
        "minecraft:redstone_block"
    )));
    // `SOUL_SAND` is `isRedstoneConductor(Blocks::always)`
    // (`Blocks.java:2093`) — its collision shape is a 14/16 column, so a
    // shape test would answer false, and vanilla answers true.
    assert!(is_redstone_conductor(default_state("minecraft:soul_sand")));
    // The default predicate (`BlockBehaviour.java:1003`) for the two obvious
    // controls, so the two overrides above are not just a constant answer.
    assert!(is_redstone_conductor(default_state("minecraft:stone")));
    assert!(!is_redstone_conductor(air()));
    assert!(!is_redstone_conductor(default_state("minecraft:glass")));
}

// ---------------------------------------------------------------------------
// The three per-state members
// ---------------------------------------------------------------------------

#[test]
fn a_redstone_block_is_a_source_of_fifteen_on_every_side_and_directs_none() {
    let redstone_block = default_state("minecraft:redstone_block");
    // `PoweredBlock.isSignalSource` (`PoweredBlock.java:23-25`).
    assert!(is_signal_source(redstone_block, WireSignal::Emitting));
    let mut read = |_: BlockPos| air();
    let mut context = SignalContext::emitting(&mut read);
    for direction in DIRECTIONS {
        // `PoweredBlock.getSignal` (`PoweredBlock.java:28-30`) ignores the
        // side entirely.
        assert_eq!(
            signal(redstone_block, ORIGIN, direction, &mut context),
            SIGNAL_MAX
        );
        // `PoweredBlock` declares no `getDirectSignal` at all, so the block
        // takes `BlockBehaviour`'s default (`BlockBehaviour.java:380-382`).
        // This is the whole reason a redstone block does not power the block
        // beside it.
        assert_eq!(
            direct_signal(redstone_block, ORIGIN, direction, &mut context),
            0
        );
    }
}

#[test]
fn a_block_with_no_override_takes_the_empty_defaults() {
    for name in [
        "minecraft:stone",
        "minecraft:glass",
        "minecraft:redstone_lamp",
    ] {
        let state = default_state(name);
        // `BlockBehaviour.isSignalSource` (`BlockBehaviour.java:233-235`).
        assert!(!is_signal_source(state, WireSignal::Emitting), "{name}");
        // `BlockBehaviour.getSignal` (`:371-373`) and `getDirectSignal`
        // (`:380-382`).
        let mut read = |_: BlockPos| air();
        let mut context = SignalContext::emitting(&mut read);
        assert_eq!(
            signal(state, ORIGIN, Direction::Up, &mut context),
            0,
            "{name}"
        );
        assert_eq!(
            direct_signal(state, ORIGIN, Direction::Up, &mut context),
            0,
            "{name}"
        );
    }
}

/// `JukeboxBlock.getSignal` (`JukeboxBlock.java:100-102`) reads its song
/// player, not its state: a jukebox holding a record it is not playing is as
/// dark as an empty one, and both directions answer alike.
#[test]
fn a_jukebox_powers_every_face_while_its_song_player_runs() {
    let jukebox = default_state("minecraft:jukebox");
    assert!(is_signal_source(jukebox, WireSignal::Emitting));
    let mut read = |_: BlockPos| air();

    let silent = |_: BlockPos| false;
    let silent_sources = super::TestSignalSources {
        jukebox_playing_at: Some(&silent),
        ..Default::default()
    };
    let mut context = SignalContext::emitting(&mut read).with_reader(&silent_sources);
    assert_eq!(signal(jukebox, ORIGIN, Direction::Up, &mut context), 0);

    let running = |pos: BlockPos| pos == ORIGIN;
    let running_sources = super::TestSignalSources {
        jukebox_playing_at: Some(&running),
        ..Default::default()
    };
    let mut context = SignalContext::emitting(&mut read).with_reader(&running_sources);
    for direction in DIRECTION_VALUES {
        assert_eq!(
            signal(jukebox, ORIGIN, direction, &mut context),
            15,
            "{direction:?}"
        );
        // `getDirectSignal` is not overridden, so it stays
        // `BlockBehaviour`'s zero (`BlockBehaviour.java:380-382`).
        assert_eq!(direct_signal(jukebox, ORIGIN, direction, &mut context), 0);
    }
}

#[test]
fn state_only_analog_outputs_follow_the_block_state() {
    assert_eq!(
        state_only_analog_output(default_state("minecraft:cauldron")),
        Some(0)
    );
    assert_eq!(
        state_only_analog_output(default_state("minecraft:lava_cauldron")),
        Some(3)
    );
    for level in 1..=3 {
        let state = with_property(
            default_state("minecraft:water_cauldron"),
            "level",
            &level.to_string(),
        )
        .expect("water cauldron level");
        assert_eq!(state_only_analog_output(state), Some(level));
    }
    for eye in [false, true] {
        let state = with_property(
            default_state("minecraft:end_portal_frame"),
            "eye",
            if eye { "true" } else { "false" },
        )
        .expect("end portal frame eye");
        assert_eq!(
            state_only_analog_output(state),
            Some(if eye { 15 } else { 0 })
        );
    }
    for (charge, expected) in [0, 3, 7, 11, 15].into_iter().enumerate() {
        let state = with_property(
            default_state("minecraft:respawn_anchor"),
            "charges",
            &charge.to_string(),
        )
        .expect("respawn anchor charge");
        assert_eq!(state_only_analog_output(state), Some(expected));
    }
    assert_eq!(
        state_only_analog_output(default_state("minecraft:candle_cake")),
        Some(14)
    );
}

#[test]
fn diode_signals_follow_power_and_the_comparator_output() {
    let mut read = |_: BlockPos| air();
    let mut context = SignalContext::emitting(&mut read);
    // The three members answer independently, and the comparator is where
    // that is visible. `DiodeBlock.isSignalSource` (`DiodeBlock.java:149-151`)
    // is a flat `true` both subclasses take.
    let comparator = default_state("minecraft:comparator");
    assert!(is_signal_source(comparator, WireSignal::Emitting));
    // `DiodeBlock.getSignal` (`:76-77`) answers a real zero while the diode is
    // off, whichever subclass it is -- the output is never asked for.
    assert_eq!(
        signal(comparator, ORIGIN, Direction::North, &mut context),
        0
    );
    // Powered and facing the asker, it reads `getOutputSignal`, which
    // `ComparatorBlock` replaces with a `ComparatorBlockEntity` read
    // (`ComparatorBlock.java:68-70`). No object bound here is vanilla's 0.
    let powered_comparator = state_of(
        "minecraft:comparator",
        &[
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "true"),
        ],
    );
    assert_eq!(
        signal(powered_comparator, ORIGIN, Direction::North, &mut context),
        0
    );
    // A repeater in the very same state is answered, because
    // `RepeaterBlock` takes `DiodeBlock.getOutputSignal`'s flat fifteen
    // (`DiodeBlock.java:189-191`).
    let powered_repeater = state_of(
        "minecraft:repeater",
        &[
            ("delay", "1"),
            ("facing", "north"),
            ("locked", "false"),
            ("powered", "true"),
        ],
    );
    assert_eq!(
        signal(powered_repeater, ORIGIN, Direction::North, &mut context),
        SIGNAL_MAX
    );
    assert_eq!(
        direct_signal(powered_repeater, ORIGIN, Direction::North, &mut context),
        SIGNAL_MAX
    );
    let output = |_: BlockPos| SIGNAL_MAX;
    let output_sources = super::TestSignalSources {
        comparator_output_at: Some(&output),
        ..Default::default()
    };
    let mut bound = SignalContext::emitting(&mut read).with_reader(&output_sources);
    assert_eq!(
        signal(powered_comparator, ORIGIN, Direction::North, &mut bound),
        SIGNAL_MAX
    );
}

// ---------------------------------------------------------------------------
// `getSignal` and the conductor fold
// ---------------------------------------------------------------------------

#[test]
fn has_signal_is_get_signal_above_zero() {
    // `SignalGetter.hasSignal` (`:61-63`).
    let mut probe = Probe::new(&[(ORIGIN, default_state("minecraft:redstone_block"))]);
    let mut read = |pos| probe.read(pos);
    assert!(has_signal(
        ORIGIN,
        Direction::Up,
        &mut SignalContext::emitting(&mut read)
    ));
    assert!(!has_signal(
        relative(ORIGIN, Direction::Up),
        Direction::Up,
        &mut SignalContext::emitting(&mut read)
    ));
}

#[test]
fn a_redstone_block_powers_the_cell_beside_it_but_not_through_a_conductor() {
    // The redstone block at `ORIGIN`, a stone block east of it, and the cell
    // east of the stone.
    let source = ORIGIN;
    let stone = relative(source, Direction::East);
    let beyond = relative(stone, Direction::East);
    let world = [
        (source, default_state("minecraft:redstone_block")),
        (stone, default_state("minecraft:stone")),
    ];

    // The cell west of the source sees it: `getSignal` on the redstone block
    // is `PoweredBlock.getSignal`, a flat 15.
    let mut probe = Probe::new(&world);
    let answer = {
        let mut read = |pos| probe.read(pos);
        has_neighbor_signal(
            relative(source, Direction::West),
            &mut SignalContext::emitting(&mut read),
        )
    };
    assert!(answer);

    // The stone is *not* powered by it. `getSignal(stone)` folds in
    // `getDirectSignalTo(stone)`, which asks the redstone block for its
    // *direct* signal — and `PoweredBlock` declares none, so it is 0.
    let mut probe = Probe::new(&world);
    let answer = {
        let mut read = |pos| probe.read(pos);
        get_signal(
            stone,
            Direction::East,
            &mut SignalContext::emitting(&mut read),
        )
    };
    assert_eq!(answer, 0, "a redstone block is not a strong source");

    // So nothing on the far side of the stone sees anything either.
    let mut probe = Probe::new(&world);
    let answer = {
        let mut read = |pos| probe.read(pos);
        has_neighbor_signal(beyond, &mut SignalContext::emitting(&mut read))
    };
    assert!(!answer);
}

// ---------------------------------------------------------------------------
// The three six-sided walks
// ---------------------------------------------------------------------------

#[test]
fn has_neighbor_signal_sees_a_source_on_any_side() {
    // `SignalGetter.hasNeighborSignal` (`:71-83`).
    let below = relative(ORIGIN, Direction::Down);
    let mut probe = Probe::new(&[(below, default_state("minecraft:redstone_block"))]);
    let answer = {
        let mut read = |pos| probe.read(pos);
        has_neighbor_signal(ORIGIN, &mut SignalContext::emitting(&mut read))
    };
    assert!(answer);

    let east = relative(ORIGIN, Direction::East);
    let mut probe = Probe::new(&[(east, default_state("minecraft:redstone_block"))]);
    let answer = {
        let mut read = |pos| probe.read(pos);
        has_neighbor_signal(ORIGIN, &mut SignalContext::emitting(&mut read))
    };
    assert!(answer);

    // Six air sides: the answer is false.
    let mut probe = Probe::new(&[]);
    let answer = {
        let mut read = |pos| probe.read(pos);
        has_neighbor_signal(ORIGIN, &mut SignalContext::emitting(&mut read))
    };
    assert!(!answer);
}

#[test]
fn get_best_neighbor_signal_is_the_strongest_side() {
    // `SignalGetter.getBestNeighborSignal` (`:85-100`).
    let north = relative(ORIGIN, Direction::North);
    let mut probe = Probe::new(&[(north, default_state("minecraft:redstone_block"))]);
    let answer = {
        let mut read = |pos| probe.read(pos);
        get_best_neighbor_signal(ORIGIN, &mut SignalContext::emitting(&mut read))
    };
    assert_eq!(answer, SIGNAL_MAX);

    // With nothing powered, the best is zero.
    let mut probe = Probe::new(&[]);
    let answer = {
        let mut read = |pos| probe.read(pos);
        get_best_neighbor_signal(ORIGIN, &mut SignalContext::emitting(&mut read))
    };
    assert_eq!(answer, 0);
}

#[test]
fn get_direct_signal_to_ignores_a_weak_only_source() {
    // `SignalGetter.getDirectSignalTo` (`:17-46`).
    let mut probe = Probe::new(&[(
        relative(ORIGIN, Direction::Down),
        default_state("minecraft:redstone_block"),
    )]);
    let answer = {
        let mut read = |pos| probe.read(pos);
        get_direct_signal_to(ORIGIN, &mut SignalContext::emitting(&mut read))
    };
    // The redstone block below is a *signal* source and not a *direct* one,
    // so it contributes nothing here.
    assert_eq!(answer, 0);
}

// ---------------------------------------------------------------------------
// `getControlInputSignal`
// ---------------------------------------------------------------------------

#[test]
fn get_control_input_signal_hard_codes_the_redstone_block_and_the_wire() {
    // `:52-53`: the redstone block answers 15 even though its own
    // `getDirectSignal` is 0, which is exactly why the branch exists.
    let mut probe = Probe::new(&[(ORIGIN, default_state("minecraft:redstone_block"))]);
    let mut read = |pos| probe.read(pos);
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            false,
            &mut SignalContext::emitting(&mut read)
        ),
        SIGNAL_MAX
    );

    // `:54-55`: redstone wire answers its `POWER` property
    // (`RedStoneWireBlock.java:50`) rather than either signal member. The
    // difference is visible now that the wire *has* a body: `getSignal` would
    // have refused the north side of a wire with no connection there, and this
    // branch hands back the stored strength without asking.
    let wire = with_property(default_state("minecraft:redstone_wire"), "power", "7")
        .expect("every wire state carries POWER");
    let mut probe = Probe::new(&[(ORIGIN, wire)]);
    let mut read = |pos| probe.read(pos);
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            false,
            &mut SignalContext::emitting(&mut read)
        ),
        7
    );

    // `:57`: anything else falls through to `isSignalSource`, and a block
    // that is not one answers zero without reading a direct signal.
    let mut probe = Probe::new(&[(ORIGIN, default_state("minecraft:stone"))]);
    let mut read = |pos| probe.read(pos);
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            false,
            &mut SignalContext::emitting(&mut read)
        ),
        0
    );
}

#[test]
fn only_a_diode_feeds_a_diodes_only_side_input() {
    // `:50-51`: with `onlyDiodes` the method answers 0 for anything that is
    // not a `DiodeBlock` — including the redstone block, whose hard-coded 15
    // is in the branch below it.
    let mut probe = Probe::new(&[(ORIGIN, default_state("minecraft:redstone_block"))]);
    let mut read = |pos| probe.read(pos);
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            true,
            &mut SignalContext::emitting(&mut read)
        ),
        0
    );

    // A repeater *is* a diode, so the call reaches `getDirectSignal` and gets
    // the diode's own fifteen -- which is the whole point of the gate: this is
    // the read that locks the repeater standing beside it.
    let powered_repeater = state_of(
        "minecraft:repeater",
        &[
            ("delay", "1"),
            ("facing", "north"),
            ("locked", "false"),
            ("powered", "true"),
        ],
    );
    let mut probe = Probe::new(&[(ORIGIN, powered_repeater)]);
    let mut read = |pos| probe.read(pos);
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            true,
            &mut SignalContext::emitting(&mut read)
        ),
        SIGNAL_MAX
    );

    // The comparator is a diode too. With no `ComparatorBlockEntity` bound it
    // answers 0, which is vanilla's missing-object `getOutputSignal`
    // (`ComparatorBlock.java:68-70`); a live output of fifteen locks the
    // repeater the same way a powered repeater does.
    let powered_comparator = state_of(
        "minecraft:comparator",
        &[
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "true"),
        ],
    );
    let mut probe = Probe::new(&[(ORIGIN, powered_comparator)]);
    let mut read = |pos| probe.read(pos);
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            true,
            &mut SignalContext::emitting(&mut read)
        ),
        0
    );
    let output = |_: BlockPos| SIGNAL_MAX;
    let output_sources = super::TestSignalSources {
        comparator_output_at: Some(&output),
        ..Default::default()
    };
    assert_eq!(
        get_control_input_signal(
            ORIGIN,
            Direction::North,
            true,
            &mut SignalContext::emitting(&mut read).with_reader(&output_sources)
        ),
        SIGNAL_MAX
    );
}

/// A shulker box is `dynamicShape` with the default `isRedstoneConductor`
/// (`Blocks.java:6973-6982`), so a lid off the full-block shape stops the
/// powered-block fold (`SignalGetter.java:68`).
#[test]
fn an_open_shulker_box_does_not_conduct() {
    let shulker = default_state("minecraft:shulker_box");
    let lever = state_of(
        "minecraft:lever",
        &[("face", "wall"), ("facing", "east"), ("powered", "true")],
    );
    let lever_pos = relative(ORIGIN, Direction::East);
    let mut probe = Probe::new(&[(ORIGIN, shulker), (lever_pos, lever)]);
    let mut read = |pos: BlockPos| probe.read(pos);

    let closed = |_: BlockPos| false;
    let closed_sources = super::TestSignalSources {
        shulker_open_at: Some(&closed),
        ..Default::default()
    };
    let mut context = SignalContext::emitting(&mut read).with_reader(&closed_sources);
    assert_eq!(get_signal(ORIGIN, Direction::West, &mut context), 15);

    let open = |pos: BlockPos| pos == ORIGIN;
    let open_sources = super::TestSignalSources {
        shulker_open_at: Some(&open),
        ..Default::default()
    };
    let mut context = SignalContext::emitting(&mut read).with_reader(&open_sources);
    assert_eq!(get_signal(ORIGIN, Direction::West, &mut context), 0);
}
