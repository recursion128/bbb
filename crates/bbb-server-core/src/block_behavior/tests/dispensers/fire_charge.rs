use super::*;

/// `FireChargeItem.asProjectile` (`FireChargeItem.java:70-71`): the small
/// fireball carries the dispensed stack, components included.
#[test]
fn a_dispensed_fire_charge_carries_its_stack() {
    let mut charge = stack("minecraft:fire_charge", 3);
    charge
        .component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_REPAIR_COST_TYPE_ID);
    charge.component_patch.repair_cost = Some(5);
    let mut world = NativeWorld::ticking(DROPPER);
    fire(&mut world, charge);
    let fireball = world
        .entities()
        .into_iter()
        .find_map(|entity| match entity.data() {
            crate::entity::EntityData::SmallFireball(data) => Some(data.item().clone()),
            _ => None,
        })
        .expect("the fire charge is shot");
    assert_eq!(fireball.summary().count, 1);
    assert_eq!(fireball.summary().component_patch.repair_cost, Some(5));
}
