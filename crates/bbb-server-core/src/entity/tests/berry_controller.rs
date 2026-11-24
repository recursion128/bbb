use super::environment::{joined_level, unoccupied_ids};
use super::*;
use crate::connection::ConnectionId;
use crate::entity::inside_blocks::berry;
use crate::entity::tests::native_level::NativeLevel;
use bbb_protocol::equipment_slots::EquipmentSlot;

const FEET: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
const CLIENT: Vec3d = Vec3d {
    x: 0.01,
    y: 0.0,
    z: 0.0,
};

fn spawn(store: &mut EntityStore, resource: &str) -> i32 {
    store
        .spawn_general_living(
            vanilla_entity_type_id_for_resource_id(resource).unwrap(),
            FEET,
        )
        .unwrap()
}
fn fixture(hand: Option<&str>, movement: bool) -> (EntityStore, i32, NativeLevel) {
    let (mut native, sender, receiver) = joined_level();
    if let Some(slot) = hand {
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut native.world,
            1,
            &format!("item replace entity alice weapon.{slot} with minecraft:carrot_on_a_stick"),
        );
    }
    if movement {
        native
            .world
            .set_player_position_for_test(ConnectionId(1), FEET);
        sender
            .try_send(crate::world::WorldCommand::Play {
                connection_id: ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::MovePlayer(
                    bbb_protocol::packets::MovePlayer {
                        position: Some(Vec3d {
                            x: FEET.x + CLIENT.x,
                            ..FEET
                        }),
                        rotation: None,
                        on_ground: true,
                        horizontal_collision: false,
                    },
                ),
            })
            .unwrap();
        native.world.tick(receiver.drain());
    }
    let mut store = EntityStore::new(17, unoccupied_ids(&native));
    let pig = spawn(&mut store, "minecraft:pig");
    store
        .get_mut(pig)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::Saddle,
            crate::entity::item::plain_stack("minecraft:saddle", 1),
        );
    assert!(store.attach_test_player(
        native.world.player_entity_id_for_test(ConnectionId(1)),
        pig,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
    (store, pig, native)
}
fn inside(store: &mut EntityStore, id: i32, native: &mut NativeLevel) -> f32 {
    let state = state_with("minecraft:sweet_berry_bush", &[("age", "3")]);
    let mut body = store.detach(id).unwrap();
    let before = body.living().unwrap().health();
    assert!(native
        .with_level(|level| berry::entity_inside(
            &mut body,
            store,
            level,
            state,
            &mut crate::entity::general_living::hurt_from_environment_outcome
        ))
        .is_some());
    let lost = before - body.living().unwrap().health();
    store.attach(body);
    lost
}

#[test]
fn stored_berry_direct_player_input_overrides_placement_and_reads_both_hands() {
    for (hand, movement, hurt) in [
        (Some("mainhand"), false, 0.0),
        (Some("mainhand"), true, 1.0),
        (Some("offhand"), true, 1.0),
        (None, true, 0.0),
    ] {
        let (mut store, pig, mut native) = fixture(hand, movement);
        store
            .get_mut(pig)
            .unwrap()
            .set_pos(Vec3d { x: 0.52, ..FEET });
        if movement {
            store.get_mut(pig).unwrap().set_old_pos_and_rot();
        }
        assert_eq!(
            inside(&mut store, pig, &mut native),
            hurt,
            "hand={hand:?}, moving={movement}"
        );
    }
    let (mut store, pig, mut native) = fixture(None, false);
    store
        .get_mut(pig)
        .unwrap()
        .set_pos(Vec3d { x: 0.55, ..FEET });
    assert_eq!(inside(&mut store, pig, &mut native), 1.0);
}

#[test]
fn stored_berry_authority_follows_mob_chain_but_movement_uses_only_direct_player() {
    let (mut store, pig, mut native) = fixture(Some("mainhand"), false);
    let cow = spawn(&mut store, "minecraft:cow");
    assert!(store.link_existing_passenger_forced(cow, pig));
    let body = store.detach(cow).unwrap();
    assert_eq!(
        native.with_level(|level| berry::movement(&body, &store, level)),
        Some(Vec3d::default())
    );
    store.attach(body);
    store.get_mut(cow).unwrap().set_delta_movement(CLIENT);
    assert_eq!(
        native.with_level(|level| berry::movement(store.get(cow).unwrap(), &store, level)),
        Some(CLIENT)
    );
    store
        .get_mut(cow)
        .unwrap()
        .set_delta_movement(Vec3d::default());
    assert_eq!(
        native.with_level(|level| berry::movement(store.get(cow).unwrap(), &store, level)),
        Some(Vec3d::default())
    );
    assert!(store.stop_invalid_riding(pig));
    let boat = crate::entity::spawn_boat(
        &mut store,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        FEET,
        0.0,
    )
    .unwrap();
    assert!(store.link_existing_passenger_forced(boat, pig));
    store.get_mut(boat).unwrap().set_delta_movement(CLIENT);
    assert_eq!(
        native.with_level(|level| berry::movement(store.get(boat).unwrap(), &store, level)),
        Some(CLIENT)
    );
}

#[test]
fn stored_berry_contact_uses_current_delta_for_damage() {
    for (delta, hurt) in [(CLIENT, 1.0), (Vec3d::default(), 0.0)] {
        let (mut store, pig, mut native) = fixture(Some("mainhand"), false);
        let cow = spawn(&mut store, "minecraft:cow");
        assert!(store.link_existing_passenger_forced(cow, pig));
        store.get_mut(cow).unwrap().set_delta_movement(delta);
        assert_eq!(inside(&mut store, cow, &mut native), hurt);
    }
}
