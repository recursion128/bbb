use bbb_protocol::entity_synched_data::{
    VANILLA_CAT_COLLAR_COLOR_DATA_ID, VANILLA_WOLF_COLLAR_COLOR_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};

use super::*;
use crate::entity::{EntityIdCounter, EntityStore};

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

/// `Cat` and `Wolf` are the whole set of bodies that declare a collar
/// (`Cat.java:76`, `Wolf.java:89`).
#[test]
fn the_collar_reads_its_saved_byte_back() {
    // `input.read("CollarColor", ..).orElse(DEFAULT_COLLAR_COLOR)`
    // (`Cat.java:224`, `Wolf.java:245`).
    assert_eq!(
        read_save_data(VANILLA_ENTITY_TYPE_WOLF_ID, &[])
            .unwrap()
            .color(),
        DEFAULT_COLLAR_COLOR
    );
    let lime = read_save_data(
        VANILLA_ENTITY_TYPE_CAT_ID,
        &[entry("CollarColor", NbtSummaryValue::Byte(5))],
    )
    .unwrap();
    assert_eq!(lime.color(), 5);

    // `ByIdMap.continuous(.., OutOfBoundsStrategy.ZERO)` (`DyeColor.java:48`).
    for outside in [-1i8, 16, 127] {
        let state = read_save_data(
            VANILLA_ENTITY_TYPE_CAT_ID,
            &[entry("CollarColor", NbtSummaryValue::Byte(outside))],
        )
        .unwrap();
        assert_eq!(state.color(), 0, "{outside} reads as WHITE");
    }
}

/// The two writes the accessor owes a client and a save file: each body's own
/// synched row (`Cat.java:204`, `Wolf.java:219`) and the `CollarColor` byte
/// (`Cat.java:214`, `Wolf.java:231`).
#[test]
fn a_dyed_collar_publishes_its_own_row_and_saves_its_legacy_byte() {
    let mut store = EntityStore::new(0xC0_11A2, EntityIdCounter::new());
    for (entity_type_id, data_id) in [
        (VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_CAT_COLLAR_COLOR_DATA_ID),
        (
            VANILLA_ENTITY_TYPE_WOLF_ID,
            VANILLA_WOLF_COLLAR_COLOR_DATA_ID,
        ),
    ] {
        let id = store
            .spawn_general_living(
                entity_type_id,
                Vec3d {
                    x: 0.5,
                    y: 64.0,
                    z: 0.5,
                },
            )
            .expect("a stored body");
        let state = color_of(store.get(id).expect("the body")).expect("a collared body");
        assert_eq!(state, DEFAULT_COLLAR_COLOR);

        let rows = synched_values(
            entity_type_id,
            CollarRetainedState::from_saved(DEFAULT_COLLAR_COLOR),
        )
        .expect("a collared body publishes one row");
        assert_eq!(rows[0].value.data_id, data_id);
        assert_eq!(rows[0].value.value, EntityDataValueKind::Int(14));
        assert!(rows[0].is_default);

        // `DyeColor.BLACK` is 15.
        assert!(crate::entity::collar::set_collar_color(
            store.get_mut(id).expect("the body"),
            15
        ));
        let rows = synched_values(
            entity_type_id,
            CollarRetainedState::from_saved(color_of(store.get(id).expect("the body")).unwrap()),
        )
        .expect("a collared body publishes one row");
        assert_eq!(rows[0].value.value, EntityDataValueKind::Int(15));
        assert!(!rows[0].is_default);

        let mut entries = Vec::new();
        write_save_data(
            &mut entries,
            CollarRetainedState::from_saved(color_of(store.get(id).expect("the body")).unwrap()),
        );
        assert_eq!(entries, [entry("CollarColor", NbtSummaryValue::Byte(15))]);
    }

    // A body with no collar declares no row at all.
    assert!(synched_values(
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        CollarRetainedState::from_saved(0)
    )
    .is_none());
}
