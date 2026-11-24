//! `ChiseledBookShelfBlock` identity and `SelectableSlotContainer.getHitSlot`.

use bbb_protocol::packets::Direction;

use super::state_of;

fn bookshelf(facing: &str) -> i32 {
    state_of(
        "minecraft:chiseled_bookshelf",
        &[
            ("facing", facing),
            ("slot_0_occupied", "false"),
            ("slot_1_occupied", "false"),
            ("slot_2_occupied", "false"),
            ("slot_3_occupied", "false"),
            ("slot_4_occupied", "false"),
            ("slot_5_occupied", "false"),
        ],
    )
}

/// `SelectableSlotContainer.getHitSlot` (`SelectableSlotContainer.java:17-23`).
///
/// The client prediction path already pins NORTH cursor `[0.9, 0.75, 0.0]` to
/// slot 0; this is the same UV fold the server click uses.
#[test]
fn a_north_face_cursor_hits_slot_zero() {
    use crate::block_behavior::chiseled_bookshelf::hit_slot;
    assert_eq!(
        hit_slot(Direction::North, Direction::North, [0.9, 0.75, 0.0]),
        Some(0)
    );
    assert_eq!(
        hit_slot(Direction::North, Direction::Up, [0.9, 0.75, 0.0]),
        None
    );
}

/// `ChiseledBookShelfBlock.updateState` occupancy bits (`:40-44`).
#[test]
fn occupancy_follows_the_six_slots() {
    use crate::block_behavior::chiseled_bookshelf::{slot_occupied, with_occupancy};
    use bbb_protocol::item_types::VANILLA_ITEM_BOOK_ID;
    use bbb_protocol::packets::ItemStackSummary;

    let empty = vec![ItemStackSummary::empty(); 6];
    let start = bookshelf("north");
    let vacant = with_occupancy(start, &empty).expect("occupancy is a 26.1 property");
    assert!(!slot_occupied(vacant, 0));

    let mut items = empty;
    items[0] = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_BOOK_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let occupied = with_occupancy(start, &items).expect("occupancy is a 26.1 property");
    assert!(slot_occupied(occupied, 0));
    assert!(!slot_occupied(occupied, 1));
}
