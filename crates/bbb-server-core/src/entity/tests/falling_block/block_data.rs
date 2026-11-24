use super::*;

fn payload() -> Vec<bbb_protocol::NbtSummaryEntry> {
    vec![bbb_protocol::NbtSummaryEntry {
        name: "nested".into(),
        value: NbtSummaryValue::Compound(vec![
            bbb_protocol::NbtSummaryEntry {
                name: "double".into(),
                value: NbtSummaryValue::Double(1.5_f64.to_bits()),
            },
            bbb_protocol::NbtSummaryEntry {
                name: "array".into(),
                value: NbtSummaryValue::IntArray(vec![i32::MIN, 42, i32::MAX]),
            },
        ]),
    }]
}

#[test]
fn falling_block_data_save_reload_distinguishes_null_empty_and_nested_payload() {
    for payload in [None, Some(vec![]), Some(payload())] {
        let mut store = EntityStore::new(7, EntityIdCounter::new());
        let id = store.spawn(
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
            EntityData::FallingBlock(
                FallingBlockData::new(state_id("minecraft:chest"), pos(0, 64, 0))
                    .with_block_data(payload.clone()),
            ),
        );
        let saved = write_entity_chunk(&store, (0, 0)).unwrap();
        let mut loaded = EntityStore::new(7, EntityIdCounter::new());
        let loaded_id = read_entity_chunk(&mut loaded, &saved).unwrap()[0];
        assert_eq!(
            falling_block_state(&loaded, loaded_id).block_data(),
            payload.as_deref()
        );
        assert_eq!(
            falling_block_state(&store, id).block_data(),
            payload.as_deref()
        );
    }
}

#[test]
fn falling_block_data_command_load_replaces_persistent_payload_but_keeps_start_pos() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let start = pos(0, 64, 0);
    let id = store.spawn(
        VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        Vec3d {
            x: 0.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::FallingBlock(
            FallingBlockData::new(state_id("minecraft:chest"), start)
                .with_block_data(Some(payload())),
        ),
    );
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    entries
        .iter_mut()
        .find(|entry| entry.name == "TileEntityData")
        .unwrap()
        .value = NbtSummaryValue::Compound(vec![]);
    store.load_command_data(id, &entries).unwrap();
    assert_eq!(falling_block_state(&store, id).block_data(), Some(&[][..]));
    assert_eq!(falling_block_state(&store, id).start_pos(), start);
    entries.retain(|entry| entry.name != "TileEntityData");
    store.load_command_data(id, &entries).unwrap();
    assert_eq!(falling_block_state(&store, id).block_data(), None);
}
