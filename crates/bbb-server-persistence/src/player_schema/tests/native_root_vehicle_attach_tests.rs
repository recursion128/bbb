use super::*;

#[test]
fn raw_attach_is_retained_for_after_tree_native_read() {
    let player_uuid = Uuid::from_u128(7);
    let root_uuid = Uuid::from_u128(8);
    for attach in [
        None,
        Some(NbtSummaryValue::String("bad UUID".into())),
        Some(uuid_array(Uuid::from_u128(9))),
    ] {
        let mut wrapper = vec![e(
            "Entity",
            NbtSummaryValue::Compound(embedded_entity_entries(root_uuid, "minecraft:oak_boat")),
        )];
        if let Some(attach) = attach {
            wrapper.push(e("Attach", attach));
        }
        let mut raw = canonical(player_uuid);
        raw.entries_mut()
            .push(e("RootVehicle", NbtSummaryValue::Compound(wrapper)));
        let original = raw.clone();
        let parsed = Java26PlayerData::parse(raw, player_uuid)
            .expect("Attach belongs to the Native consumer after root construction");
        assert_eq!(parsed.raw(), &original);
        assert_eq!(parsed.root_vehicle().unwrap().entity().uuid(), root_uuid);
    }
}
