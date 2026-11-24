use bbb_protocol::block_types::{VANILLA_BLOCK_NOTE_BLOCK_ID, VANILLA_BLOCK_PISTON_ID};

use super::*;

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

fn note_block_event(at: BlockPos, param_a: i32, param_b: i32) -> BlockEventData {
    BlockEventData::new(at, VANILLA_BLOCK_NOTE_BLOCK_ID, param_a, param_b)
}

#[test]
fn an_identical_event_enqueued_twice_is_kept_once() {
    let event = note_block_event(pos(1, 2, 3), 0, 5);
    let mut queue = BlockEventQueue::new();

    assert!(queue.block_event(event));
    assert!(!queue.block_event(event));

    assert_eq!(queue.len(), 1);
    assert_eq!(queue.iter().collect::<Vec<_>>(), vec![event]);
}

#[test]
fn an_event_differing_in_one_parameter_is_a_separate_queue_entry() {
    let at = pos(1, 2, 3);
    let mut queue = BlockEventQueue::new();

    assert!(queue.block_event(note_block_event(at, 0, 5)));
    assert!(queue.block_event(note_block_event(at, 0, 6)));
    assert!(queue.block_event(note_block_event(at, 1, 5)));
    assert!(queue.block_event(BlockEventData::new(at, VANILLA_BLOCK_PISTON_ID, 0, 5)));
    assert!(queue.block_event(note_block_event(pos(1, 2, 4), 0, 5)));

    assert_eq!(queue.len(), 5);
}

#[test]
fn a_broadcast_reaches_a_viewer_just_inside_sixty_four_blocks() {
    let event = note_block_event(pos(0, 64, 0), 0, 5);

    assert!(event.reaches(true, 0.0, 64.0, 0.0));
    assert!(event.reaches(true, 63.9, 64.0, 0.0));
    assert!(event.reaches(true, 36.0, 64.0, 36.0));
}

#[test]
fn a_broadcast_stops_exactly_at_sixty_four_blocks() {
    // `xd * xd + yd * yd + zd * zd < range * range` (PlayerList.java:612) is
    // strict, so a viewer sitting exactly 64 blocks away gets nothing.
    let event = note_block_event(pos(0, 64, 0), 0, 5);

    assert!(!event.reaches(true, 64.0, 64.0, 0.0));
    assert!(!event.reaches(true, 0.0, 0.0, 0.0));
    assert!(!event.reaches(true, 46.0, 64.0, 46.0));
}

#[test]
fn a_broadcast_never_leaves_the_events_dimension() {
    let event = note_block_event(pos(0, 64, 0), 0, 5);

    assert!(!event.reaches(false, 0.0, 64.0, 0.0));
}

#[test]
fn the_broadcast_packet_truncates_the_event_parameters_to_bytes() {
    // `ClientboundBlockEventPacket.write` uses `writeByte` on two ints
    // (ClientboundBlockEventPacket.java:37-38); the client reads them back
    // unsigned (:30-31).
    let event = BlockEventData::new(pos(1, 2, 3), VANILLA_BLOCK_PISTON_ID, 0x1_02, -1);

    assert_eq!(
        event.packet(),
        BlockEvent {
            pos: pos(1, 2, 3),
            b0: 2,
            b1: 255,
            block_id: VANILLA_BLOCK_PISTON_ID,
        }
    );
}
