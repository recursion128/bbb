use super::*;

fn expected(uuids: &[Uuid]) -> [Option<Uuid>; 2] {
    [
        uuids.first().copied(),
        (uuids.len() > 1).then(|| *uuids.last().unwrap()),
    ]
}

fn assert_trusted(store: &EntityStore, id: i32, uuids: &[Uuid]) {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        panic!()
    };
    let wanted = expected(uuids);
    let state = data.fox_retained().unwrap();
    assert_eq!(state.trusted, wanted);
    let values = synched_values(state);
    assert_eq!(
        values[2].value.value,
        EntityDataValueKind::OptionalLivingEntityReference(wanted[0])
    );
    assert_eq!(
        values[3].value.value,
        EntityDataValueKind::OptionalLivingEntityReference(wanted[1])
    );
    let NbtSummaryValue::Compound(fields) = store.command_data(id).unwrap() else {
        panic!()
    };
    assert_eq!(
        field(&fields, "Trusted"),
        Some(&NbtSummaryValue::List(
            wanted.into_iter().flatten().map(uuid_value).collect()
        ))
    );
}

#[test]
fn fox_cold_trusted_lists_restore() {
    for size in 0..=2 {
        let uuids: Vec<_> = (0..size).map(|i| Uuid::from_u128(100 + i)).collect();
        let mut record = default_record();
        set_field(
            &mut record,
            "Trusted",
            NbtSummaryValue::List(uuids.iter().copied().map(uuid_value).collect()),
        );
        let mut store = EntityStore::new(73, EntityIdCounter::new());
        let id = read_entity_chunk(&mut store, &chunk(record)).unwrap()[0];
        assert_trusted(&store, id, &uuids);
    }
}

#[test]
fn fox_command_trusted_updates_typed_and_tracking_slots() {
    let mut store = EntityStore::new(73, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_FOX_ID, position())
        .unwrap();
    assert!(crate::entity::configure_fox_target_goals(
        store.get_mut(id).unwrap()
    ));
    store.refresh_retained_mob_targets(id);
    let a = Uuid::from_u128(101);
    let b = Uuid::from_u128(102);
    let c = Uuid::from_u128(103);
    for uuids in [vec![a, b], vec![], vec![c, a, b, c]] {
        let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
            panic!()
        };
        set_field(
            &mut fields,
            "Trusted",
            NbtSummaryValue::List(uuids.iter().copied().map(uuid_value).collect()),
        );
        store.load_command_data(id, &fields).unwrap();
        assert_trusted(&store, id, &uuids);
    }
    let NbtSummaryValue::Compound(mut fields) = store.command_data(id).unwrap() else {
        panic!()
    };
    fields.retain(|entry| entry.name != "Trusted");
    store.load_command_data(id, &fields).unwrap();
    assert_trusted(&store, id, &[]);
}
