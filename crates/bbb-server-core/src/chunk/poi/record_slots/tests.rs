use super::*;

#[test]
fn poi_record_slots_insert_remove_and_clear() {
    let mut slots = PoiRecordSlots::new();
    for key in [relative_key([3, 5, 7]), 0, relative_key([15, 15, 15]), 0] {
        slots.insert(key);
    }
    assert_eq!(
        slots.stream_keys().collect::<Vec<_>>(),
        [0, relative_key([3, 5, 7]), relative_key([15, 15, 15])]
    );

    slots.remove(0);
    slots.remove(0);
    slots.remove(relative_key([1, 1, 1]));
    assert_eq!(
        slots.stream_keys().collect::<Vec<_>>(),
        [relative_key([3, 5, 7]), relative_key([15, 15, 15])]
    );

    slots.clear_preserving_capacity();
    assert_eq!(slots.stream_keys().count(), 0);
}
