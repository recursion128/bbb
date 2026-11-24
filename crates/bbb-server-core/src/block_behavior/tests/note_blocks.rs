use crate::random::GameRandom;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::packets::{Direction, SoundEventHolder, SoundSource};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::*;

fn default_state(name: &str) -> i32 {
    let entry = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"));
    vanilla_block_default_state_id(entry.id)
        .unwrap_or_else(|| panic!("{name} has no default state"))
}

fn note_block_state_with_power(instrument: &str, note: i32, powered: bool) -> i32 {
    state_of(
        "minecraft:note_block",
        &[
            ("instrument", instrument),
            ("note", &note.to_string()),
            ("powered", if powered { "true" } else { "false" }),
        ],
    )
}

fn note_block_state(instrument: &str, note: i32) -> i32 {
    note_block_state_with_power(instrument, note, false)
}

fn instrument_of(block_state_id: i32) -> String {
    bbb_protocol::block_state_types::vanilla_block_state_identity(block_state_id)
        .expect("a built-in state")
        .properties["instrument"]
        .clone()
}

fn note_of(block_state_id: i32) -> i32 {
    int_property(block_state_id, "note").expect("a note block carries `note`")
}

#[test]
fn a_placed_note_block_takes_its_instrument_from_the_block_below() {
    let placed = note_block::state_for_placement(air(), stone());
    assert_eq!(instrument_of(placed), "basedrum");
    assert_eq!(note_of(placed), 0, "the default note");

    let placed = note_block::state_for_placement(air(), default_state("minecraft:oak_planks"));
    assert_eq!(instrument_of(placed), "bass");

    assert_eq!(
        instrument_of(note_block::state_for_placement(air(), air())),
        "harp"
    );
}

#[test]
fn a_head_above_a_placed_note_block_beats_the_block_below() {
    let placed =
        note_block::state_for_placement(default_state("minecraft:skeleton_skull"), stone());
    assert_eq!(instrument_of(placed), "skeleton");
}

#[test]
fn a_head_below_a_placed_note_block_gives_a_harp() {
    let placed = note_block::state_for_placement(air(), default_state("minecraft:skeleton_skull"));
    assert_eq!(instrument_of(placed), "harp");
}

#[test]
fn only_a_vertical_neighbour_change_re_reads_the_instrument() {
    let mut level = NativeWorld::new();
    let pos = at(0, 64, 0);
    level.set(pos, note_block_state("harp", 3));
    level.set(relative(pos, Direction::Down), stone());

    let horizontal = level.with_level(|level| {
        update_block_shape(
            &mut BlockShapeCtx::Live(level),
            note_block_state("harp", 3),
            pos,
            Direction::North,
            relative(pos, Direction::North),
            air(),
        )
    });
    assert_eq!(instrument_of(horizontal), "harp");
    assert_eq!(note_of(horizontal), 3, "the note is never touched here");

    for direction in [Direction::Down, Direction::Up] {
        let vertical = level.with_level(|level| {
            update_block_shape(
                &mut BlockShapeCtx::Live(level),
                note_block_state("harp", 3),
                pos,
                direction,
                relative(pos, direction),
                air(),
            )
        });
        assert_eq!(instrument_of(vertical), "basedrum", "{direction:?}");
        assert_eq!(note_of(vertical), 3, "{direction:?}");
    }
}

#[test]
fn a_rising_redstone_edge_plays_once_and_stores_powered() {
    let mut world = listening_world();
    let pos = at(0, 64, 0);
    world.set(pos, note_block_state("harp", 3));
    world.set(
        relative(pos, Direction::East),
        default_state("minecraft:redstone_block"),
    );

    world.neighbor_changed(pos, block_type_of(stone()).expect("stone block type"));

    assert_eq!(
        world.state(pos),
        note_block_state_with_power("harp", 3, true)
    );

    assert_eq!(
        broadcast(&mut world),
        vec![bbb_protocol::packets::BlockEvent {
            pos,
            b0: 0,
            b1: 0,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID
        }]
    );
}

#[test]
fn a_falling_edge_only_clears_powered_and_steady_levels_are_noops() {
    let mut world = listening_world();
    let pos = at(0, 64, 0);
    world.set(pos, note_block_state_with_power("harp", 3, true));

    world.neighbor_changed(pos, block_type_of(stone()).expect("stone block type"));

    assert_eq!(world.state(pos), note_block_state("harp", 3));

    world.neighbor_changed(pos, block_type_of(stone()).expect("stone block type"));
    assert_eq!(world.state(pos), note_block_state("harp", 3), "steady low");

    world.set(
        relative(pos, Direction::East),
        default_state("minecraft:redstone_block"),
    );
    world.set(pos, note_block_state_with_power("harp", 3, true));
    world.neighbor_changed(pos, block_type_of(stone()).expect("stone block type"));
    assert_eq!(
        world.state(pos),
        note_block_state_with_power("harp", 3, true),
        "steady high"
    );
    assert!(broadcast(&mut world).is_empty());
}

#[test]
fn a_covered_tunable_note_block_still_stores_a_rising_edge() {
    let mut world = listening_world();
    let pos = at(0, 64, 0);
    world.set(pos, note_block_state("harp", 3));
    world.set(relative(pos, Direction::Up), stone());
    world.set(
        relative(pos, Direction::East),
        default_state("minecraft:redstone_block"),
    );

    world.neighbor_changed(pos, block_type_of(stone()).expect("stone block type"));

    assert_eq!(
        world.state(pos),
        note_block_state_with_power("harp", 3, true)
    );
    assert!(broadcast(&mut world).is_empty());
}

#[test]
fn a_right_click_advances_the_note_and_queues_one_block_event() {
    let mut level = listening_world();
    let pos = at(0, 64, 0);
    level.set(pos, note_block_state("harp", 0));
    level.set(relative(pos, Direction::Down), stone());

    let consumed = level.with_level(|level| {
        crate::block_behavior::note_block::use_without_item(
            level,
            pos,
            note_block_state("harp", 0),
            interaction_player(true, Direction::North),
        )
    });
    assert!(consumed);
    assert_eq!(note_of(level.state(pos)), 1);

    assert_eq!(
        broadcast(&mut level),
        vec![bbb_protocol::packets::BlockEvent {
            pos,
            b0: 0,
            b1: 0,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID
        }]
    );
}

#[test]
fn the_note_wraps_from_the_last_value_back_to_the_first() {
    let mut level = NativeWorld::new();
    let pos = at(0, 64, 0);
    level.set(pos, note_block_state("harp", 24));

    level.with_level(|level| {
        crate::block_behavior::note_block::use_without_item(
            level,
            pos,
            note_block_state("harp", 24),
            interaction_player(true, Direction::North),
        )
    });
    assert_eq!(note_of(level.state(pos)), 0);
}

#[test]
fn a_covered_tunable_note_block_advances_its_note_without_an_event() {
    let mut level = listening_world();
    let pos = at(0, 64, 0);
    level.set(pos, note_block_state("harp", 0));
    level.set(relative(pos, Direction::Up), stone());

    let consumed = level.with_level(|level| {
        crate::block_behavior::note_block::use_without_item(
            level,
            pos,
            note_block_state("harp", 0),
            interaction_player(true, Direction::North),
        )
    });
    assert!(consumed);
    assert_eq!(note_of(level.state(pos)), 1);
    assert!(broadcast(&mut level).is_empty());
}

#[test]
fn a_covered_mob_head_note_block_still_queues_its_event() {
    let mut level = listening_world();
    let pos = at(0, 64, 0);
    level.set(pos, note_block_state("skeleton", 0));
    level.set(
        relative(pos, Direction::Up),
        default_state("minecraft:skeleton_skull"),
    );

    level.with_level(|level| {
        crate::block_behavior::note_block::use_without_item(
            level,
            pos,
            note_block_state("skeleton", 0),
            interaction_player(true, Direction::North),
        )
    });

    assert_eq!(
        broadcast(&mut level),
        vec![bbb_protocol::packets::BlockEvent {
            pos,
            b0: 0,
            b1: 0,
            block_id: bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID
        }]
    );
}

fn triggered(block_state_id: i32) -> BlockTriggerOutcome {
    trigger_event(
        at(3, 4, 5),
        block_state_id,
        0,
        0,
        &GameRandom::with_seed(0),
        None,
    )
}

#[test]
fn a_note_block_answers_its_own_event_and_a_plain_block_does_not() {
    for instrument in ["harp", "basedrum", "skeleton", "piglin"] {
        assert!(
            triggered(note_block_state(instrument, 7)).triggered,
            "{instrument}"
        );
    }
    let custom = triggered(note_block_state("custom_head", 7));
    assert_eq!(custom, BlockTriggerOutcome::not_triggered());
    let with_sound = trigger_event(
        at(3, 4, 5),
        note_block_state("custom_head", 7),
        0,
        0,
        &GameRandom::with_seed(0),
        Some("minecraft:block.note_block.harp"),
    );
    assert!(with_sound.triggered);
    assert_eq!(
        with_sound
            .sound
            .expect("a custom head with a sound plays")
            .sound,
        SoundEventHolder::Direct {
            location: "minecraft:block.note_block.harp".to_string(),
            fixed_range: None,
        }
    );
    assert_eq!(triggered(stone()), BlockTriggerOutcome::not_triggered());
}

#[test]
fn a_tunable_note_block_plays_its_instrument_at_the_note_s_pitch() {
    let random = GameRandom::with_seed(0x_5EED);
    let outcome = trigger_event(
        at(3, 4, 5),
        note_block_state("harp", 24),
        0,
        0,
        &random,
        None,
    );
    assert!(outcome.triggered);
    let sound = outcome.sound.expect("a tunable instrument plays a sound");
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id("minecraft:block.note_block.harp")
                .expect("a registered sound"),
        }
    );
    assert_eq!(sound.source, SoundSource::Records);
    assert_eq!(
        (sound.position.x, sound.position.y, sound.position.z),
        (3.5, 4.5, 5.5)
    );
    assert_eq!(sound.volume, 3.0);
    assert_eq!(sound.pitch, 2.0);

    let unshifted = trigger_event(
        at(0, 0, 0),
        note_block_state("harp", 12),
        0,
        0,
        &random,
        None,
    );
    assert_eq!(unshifted.sound.expect("a sound").pitch, 1.0);
    let lowest = trigger_event(
        at(0, 0, 0),
        note_block_state("harp", 0),
        0,
        0,
        &random,
        None,
    );
    assert_eq!(lowest.sound.expect("a sound").pitch, 0.5);
}

#[test]
fn a_mob_head_note_block_plays_a_fixed_pitch_imitation() {
    let outcome = trigger_event(
        at(0, 0, 0),
        note_block_state("dragon", 24),
        0,
        0,
        &GameRandom::with_seed(1),
        None,
    );
    let sound = outcome.sound.expect("a mob head plays a sound");
    assert_eq!(sound.pitch, 1.0);
    assert_eq!(
        sound.sound,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id(
                "minecraft:block.note_block.imitate.ender_dragon"
            )
            .expect("a registered sound"),
        }
    );
}

#[test]
fn a_head_on_the_top_face_passes_instead_of_tuning_the_note() {
    let skull =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:skeleton_skull")
            .expect("a built-in item")
            .id;
    let stone_item = bbb_protocol::item_types::VANILLA_ITEM_STONE_ID;
    let state = note_block_state("harp", 0);

    assert_eq!(
        use_item_on(state, Some(skull), Direction::Up),
        BlockUseResult::Pass
    );
    assert_eq!(
        use_item_on(state, Some(skull), Direction::North),
        BlockUseResult::TryWithEmptyHand
    );
    assert_eq!(
        use_item_on(state, Some(stone_item), Direction::Up),
        BlockUseResult::TryWithEmptyHand
    );
    assert_eq!(
        use_item_on(state, None, Direction::Up),
        BlockUseResult::TryWithEmptyHand
    );
    assert_eq!(
        use_item_on(stone(), Some(skull), Direction::Up),
        BlockUseResult::TryWithEmptyHand
    );
}

fn listening_world() -> NativeWorld {
    let mut world = NativeWorld::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut world.world, 1, "listener", 1);
    crate::tests::place_player(
        &sender,
        &receiver,
        &mut world.world,
        1,
        bbb_protocol::packets::Vec3d {
            x: 0.5,
            y: 64.0,
            z: -2.0,
        },
        None,
    );
    world
}

fn broadcast(world: &mut NativeWorld) -> Vec<bbb_protocol::packets::BlockEvent> {
    world
        .world
        .tick(Vec::new())
        .events
        .into_iter()
        .filter_map(|event| match event {
            crate::world::WorldEvent::Send {
                packet: crate::connection::ClientboundPlayPacket::BlockEvent(event),
                ..
            } => Some(event),
            _ => None,
        })
        .collect()
}
