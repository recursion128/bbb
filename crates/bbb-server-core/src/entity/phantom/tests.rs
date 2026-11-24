use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
use bbb_protocol::entity_synched_data::VANILLA_PHANTOM_SIZE_DATA_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_PHANTOM_ID};
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use super::*;
use crate::entity::{EntityIdCounter, EntityStore, GeneralLivingData};

#[test]
fn phantom_targetless_runtime_retained_census_size_and_metadata_are_exact() {
    let owned = VANILLA_ENTITY_TYPES
        .iter()
        .filter_map(|facts| {
            GeneralLivingData::new(facts.id)
                .and_then(|data| data.phantom_retained())
                .map(|_| facts.id)
        })
        .collect::<Vec<_>>();
    assert_eq!(owned, [VANILLA_ENTITY_TYPE_PHANTOM_ID]);

    let mut store = EntityStore::new(0x5048_414E, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PHANTOM_ID, Vec3d::default())
        .unwrap();
    let entity = store.get_mut(id).unwrap();
    assert!(set_size(entity, 80));
    assert_eq!(entity.bb_width(), 0.9 * (1.0 + 0.15 * 64.0));
    assert_eq!(entity.bb_height(), 0.5 * (1.0 + 0.15 * 64.0));
    assert_eq!(
        entity
            .living()
            .unwrap()
            .attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID),
        70.0
    );
    assert_eq!(entity.take_synched_dirty(), [VANILLA_PHANTOM_SIZE_DATA_ID]);
    assert!(set_size(entity, 64));
    assert!(entity.take_synched_dirty().is_empty());
    let state = match entity.data() {
        EntityData::GeneralLiving(data) => data.phantom_retained().unwrap(),
        _ => unreachable!(),
    };
    let tracked = synched_values(state);
    assert_eq!(tracked[0].value.value, EntityDataValueKind::Int(64));
    assert!(!tracked[0].is_default);
}

#[test]
fn phantom_targetless_runtime_save_defaults_and_clamp_are_exact() {
    let default = read_save_data(&[]);
    assert_eq!(default.size(), 0);
    assert_eq!(default.anchor(), None);

    let state = read_save_data(&[
        NbtSummaryEntry {
            name: "anchor_pos".into(),
            value: NbtSummaryValue::IntArray(vec![1, 70, -2]),
        },
        NbtSummaryEntry {
            name: "size".into(),
            value: NbtSummaryValue::Int(-9),
        },
    ]);
    assert_eq!(state.anchor(), Some(BlockPos { x: 1, y: 70, z: -2 }));
    assert_eq!(state.size(), 0);
}
