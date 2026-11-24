use super::*;

const BLAZE_XP_REWARD: i32 = 10;

#[test]
fn the_equipment_bonus_raises_the_reward() {
    let declared = BLAZE_XP_REWARD;
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    native
        .world
        .set_player_position_for_test(crate::connection::ConnectionId(1), position());
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_BLAZE_ID).unwrap();
    data.set_no_ai(true);
    let (id, uuid) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_BLAZE_ID,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 8.5,
        },
        EntityData::GeneralLiving(data),
    );
    native
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::MainHand,
            crate::entity::item::plain_stack("minecraft:stick", 1),
        );
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        &format!("damage {uuid} 40 minecraft:generic by alice"),
    );
    assert!(
        native
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .living()
            .unwrap()
            .dead
    );
    let total: i32 = native
        .world
        .stored_experience_orbs_for_test(DIMENSION)
        .iter()
        .map(|orb| orb.2 * orb.3)
        .sum();
    assert!(
        (1..=3).contains(&(total - declared)),
        "the actual equipped mob awards its base reward plus a slot bonus: {total}"
    );
}
