use super::*;

fn origin() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

#[test]
fn block_trigger_ignoring_is_owned_by_one_entity_classification() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let bat_type = vanilla_entity_type_id_for_resource_id("minecraft:bat")
        .expect("bat is a built-in entity type");
    let cow_type = vanilla_entity_type_id_for_resource_id("minecraft:cow")
        .expect("cow is a built-in entity type");
    let bat = store
        .spawn_general_living(bat_type, origin())
        .expect("bat has generated living facts");
    let cow = store
        .spawn_general_living(cow_type, origin())
        .expect("cow has generated living facts");
    let stand = crate::entity::spawn_armor_stand(&mut store, origin()).expect("a fresh UUID");
    let marker_stand =
        crate::entity::spawn_armor_stand(&mut store, origin()).expect("a fresh UUID");
    crate::entity::armor_stand::set_marker(store.get_mut(marker_stand).unwrap(), true);
    let marker =
        crate::entity::spawn_marker(&mut store, origin(), crate::entity::MarkerData::new())
            .expect("a fresh UUID");
    let interaction = crate::entity::spawn_interaction(&mut store, origin()).expect("a fresh UUID");
    let display = crate::entity::spawn_block_display(
        &mut store,
        origin(),
        crate::entity::BlockDisplayData::new(),
    )
    .expect("a fresh UUID");
    let minecart = crate::entity::spawn_minecart(&mut store, origin(), 0.0).expect("a fresh UUID");

    for id in [bat, marker_stand, marker, interaction, display] {
        assert!(store.get(id).unwrap().is_ignoring_block_triggers());
    }
    for id in [cow, stand, minecart] {
        assert!(!store.get(id).unwrap().is_ignoring_block_triggers());
    }
}
