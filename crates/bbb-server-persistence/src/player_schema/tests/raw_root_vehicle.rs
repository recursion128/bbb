use super::*;

#[test]
fn raw_root_vehicle_keeps_noncanonical_leaf_fields_for_the_entity_reader() {
    let player_uuid = Uuid::from_u128(7);
    let root_uuid = Uuid::from_u128(8);
    let passenger_uuid = Uuid::from_u128(9);
    let mut passenger = embedded_entity_entries(passenger_uuid, "minecraft:fox");
    passenger.push(e("Sleeping", NbtSummaryValue::String("bad".into())));
    passenger.push(e("Type", NbtSummaryValue::Int(9)));
    let mut root = embedded_entity_entries(root_uuid, "minecraft:oak_boat");
    root.push(e(
        "Passengers",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(passenger)]),
    ));
    let mut raw = canonical(player_uuid);
    raw.entries_mut().push(e(
        "RootVehicle",
        root_vehicle_value(passenger_uuid, root.clone()),
    ));
    let parsed = Java26PlayerData::parse(raw, player_uuid).unwrap();
    let vehicle = parsed.root_vehicle().unwrap();
    assert_eq!(vehicle.attach().unwrap(), passenger_uuid);
    assert_eq!(vehicle.entity_entries(), root);
}

#[test]
fn raw_root_vehicle_retains_unresolved_attach_and_drops_a_duplicate_uuid_passenger() {
    let player_uuid = Uuid::from_u128(7);
    let root_uuid = Uuid::from_u128(8);
    let mut raw = canonical(player_uuid);
    raw.entries_mut().push(e(
        "RootVehicle",
        root_vehicle_value(
            Uuid::from_u128(9),
            embedded_entity_entries(root_uuid, "minecraft:oak_boat"),
        ),
    ));
    let original = raw.clone();
    let parsed = Java26PlayerData::parse(raw, player_uuid).unwrap();
    assert_eq!(
        parsed.root_vehicle().unwrap().attach().unwrap(),
        Uuid::from_u128(9)
    );
    assert_eq!(parsed.raw(), &original);

    let mut raw = canonical(player_uuid);
    raw.entries_mut().push(e(
        "RootVehicle",
        root_vehicle_value(root_uuid, nested_vehicle_entries(root_uuid, root_uuid)),
    ));
    let parsed = Java26PlayerData::parse(raw, player_uuid).unwrap();
    assert!(parsed.root_vehicle().unwrap().entity().passengers().len() == 0);
}

#[test]
fn raw_root_vehicle_attach_input_is_raw_and_written_back_unchanged() {
    let player_uuid = Uuid::from_u128(7);
    let tree = nested_vehicle_entries(Uuid::from_u128(8), Uuid::from_u128(9));
    for attach in [
        None,
        Some(NbtSummaryValue::String("bad".into())),
        Some(NbtSummaryValue::IntArray(vec![0, 0, 8])),
    ] {
        let mut wrapper = vec![e("Entity", NbtSummaryValue::Compound(tree.clone()))];
        if let Some(value) = &attach {
            wrapper.push(e("Attach", value.clone()));
        }
        let mut raw = canonical(player_uuid);
        raw.entries_mut()
            .push(e("RootVehicle", NbtSummaryValue::Compound(wrapper.clone())));
        let parsed = Java26PlayerData::parse(raw.clone(), player_uuid).unwrap();
        let vehicle = parsed.root_vehicle().unwrap();
        assert_eq!(vehicle.attach_input(), attach.as_ref());
        assert_eq!(vehicle.entity_entries(), tree);
        assert!(vehicle.attach().is_err());
        let mut patch = created_patch(player_uuid);
        patch.root_vehicle = Some(vehicle.clone());
        let written = parsed
            .raw_with_player_save_patch(&patch, SaveHolderResolver::unavailable().as_dyn())
            .unwrap();
        let mut expected = wrapper;
        expected.sort_by_key(|entry| entry.name != "Attach");
        assert_eq!(
            field_of(&written, "RootVehicle"),
            &NbtSummaryValue::Compound(expected)
        );
    }
}

#[test]
fn raw_root_vehicle_canonical_attach_constructor_and_writer_succeed() {
    let player_uuid = Uuid::from_u128(7);
    let attach = Uuid::from_u128(9);
    let tree = nested_vehicle_entries(Uuid::from_u128(8), attach);
    let vehicle = Java26RootVehicle::new(
        attach,
        crate::Java26EntityRecord::parse(&tree.clone()).unwrap(),
    );
    assert_eq!(vehicle.attach_input(), Some(&uuid_array(attach)));
    let mut patch = created_patch(player_uuid);
    patch.root_vehicle = Some(vehicle);
    let document = created_player_document(&patch).unwrap();
    assert_eq!(
        field_of(&document, "RootVehicle"),
        &root_vehicle_value(attach, tree)
    );
}
