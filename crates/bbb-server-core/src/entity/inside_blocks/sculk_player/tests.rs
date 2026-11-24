use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::entity::{EntityData, EntityIdCounter};
use bbb_protocol::entity_types::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::ItemStackSummary;

const POS: BlockPos = BlockPos { x: 0, y: 63, z: 0 };
const FEET: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn store() -> EntityStore {
    let ids = EntityIdCounter::new();
    ids.next_id();
    EntityStore::new(9, ids)
}
fn mob(store: &mut EntityStore, kind: i32) -> i32 {
    store.spawn_general_living(kind, FEET).unwrap()
}
fn arrow(store: &mut EntityStore, owner: Option<Uuid>) -> Box<Entity> {
    let id = crate::entity::arrow::spawn_arrow(
        store,
        FEET,
        Default::default(),
        crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
    )
    .unwrap();
    let mut body = store.detach(id).unwrap();
    let EntityData::Arrow(data) = &mut body.data else {
        panic!()
    };
    data.projectile.set_owner(owner);
    body
}
fn native_player() -> (NativeLevel, SculkPlayerIdentity) {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 999);
    let player = SculkPlayerIdentity {
        uuid: Uuid::from_u128(999),
        entity_id: native.world.player_entity_id_for_test(ConnectionId(1)),
    };
    (native, player)
}

fn player_item(native: &mut NativeLevel, item: &str, main: bool) {
    native.world.stock_menu_slot_for_test(
        ConnectionId(1),
        if main { 36 } else { 45 },
        ItemStackSummary {
            item_id: Some(vanilla_item_type_for_resource_id(item).unwrap().id),
            count: 1,
            component_patch: Default::default(),
        },
    );
}

#[test]
fn shrieker_projectile_nonplayer_owner_does_not_resolve_a_player() {
    let (mut native, _) = native_player();
    let mut store = store();
    let id = mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let uuid = store.get(id).unwrap().uuid();
    let body = arrow(&mut store, Some(uuid));
    assert_eq!(
        native.with_level(|level| try_get_player(&body, Some(&store), level)),
        Some(None)
    );
}

#[test]
fn shrieker_owner_resolves_actual_dead_spectator_player() {
    let (mut native, player) = native_player();
    let abilities = native.world.player_abilities_for_test(ConnectionId(1));
    native.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        bbb_protocol::game_types::GameType::Spectator,
        abilities,
    );
    native.world.set_player_dead_for_test(ConnectionId(1));
    let mut store = store();
    let body = arrow(&mut store, Some(player.uuid));
    assert_eq!(
        native.with_level(|level| try_get_player(&body, Some(&store), level)),
        Some(Some(player))
    );
}

#[test]
fn shrieker_owner_resolves_an_actual_player_in_another_dimension() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 999);
    let player = SculkPlayerIdentity {
        uuid: Uuid::from_u128(999),
        entity_id: native.world.player_entity_id_for_test(ConnectionId(1)),
    };
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "execute in minecraft:the_nether run tp alice 8 64 8",
    );
    assert_eq!(
        native
            .world
            .player_transition_state_for_test(ConnectionId(1))
            .0
            .resource_id(),
        "minecraft:the_nether"
    );
    let mut store = store();
    let body = arrow(&mut store, Some(player.uuid));
    assert_eq!(
        native.with_level(|level| try_get_player(&body, Some(&store), level)),
        Some(Some(player))
    );
}

#[test]
fn shrieker_warden_queries_bbox_edges_and_dead_bodies() {
    let mut store = store();
    let id = mob(&mut store, VANILLA_ENTITY_TYPE_WARDEN_ID);
    let mut warden = store.detach(id).unwrap();
    warden.living_mut().unwrap().set_health(0.0);
    warden.set_pos(Vec3d {
        x: 24.5 + f64::from(warden.bb_width()) / 2.0,
        ..FEET
    });
    assert_eq!(nearby_warden(&warden, Some(&store), POS), Some(false));
    warden.set_pos(Vec3d {
        x: warden.position().x - 0.0001,
        ..FEET
    });
    assert_eq!(nearby_warden(&warden, Some(&store), POS), Some(true));
}

fn mount(store: &mut EntityStore, vehicle: i32, player_id: i32) {
    assert!(store.attach_test_player(
        player_id,
        vehicle,
        crate::entity::ExternalPlayerCanRide::Allowed
    ));
}

fn vehicle(store: &mut EntityStore, kind: i32) -> i32 {
    match kind {
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID => {
            crate::entity::boat::spawn_boat(store, kind, FEET, 0.0).unwrap()
        }
        _ => mob(store, kind),
    }
}

fn prepare_saddle(body: &mut Entity) {
    body.living_mut().unwrap().set_item_slot(
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1),
    );
    let EntityData::GeneralLiving(data) = &mut body.data else {
        panic!()
    };
    if let Some(horse) = data.abstract_horse_retained_mut() {
        horse.set_tame(true);
    }
    if let Some(tame) = data.tamable_retained_mut() {
        tame.set_tame(true);
    }
}

#[test]
fn shrieker_controllers_require_actual_equipment_and_player_hand_items() {
    for kind in [
        VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ] {
        let (mut native, player) = native_player();
        let mut store = store();
        let id = vehicle(&mut store, kind);
        mount(&mut store, id, player.entity_id);
        let mut body = store.detach(id).unwrap();
        let controller = |native: &mut NativeLevel, body: &Entity| {
            native.with_level(|level| try_get_player(body, Some(&store), level))
        };
        if kind == VANILLA_ENTITY_TYPE_OAK_BOAT_ID {
            assert_eq!(controller(&mut native, &body), Some(Some(player)));
        } else if kind == VANILLA_ENTITY_TYPE_ZOMBIE_ID {
            assert_eq!(controller(&mut native, &body), Some(None));
        } else if kind == VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID {
            assert_eq!(controller(&mut native, &body), Some(None));
            body.living_mut().unwrap().set_item_slot(
                EquipmentSlot::Body,
                crate::entity::item::plain_stack("minecraft:white_harness", 1),
            );
            assert_eq!(controller(&mut native, &body), Some(Some(player)));
            crate::entity::happy_ghast::set_server_still_timeout(&mut body, 10);
            assert_eq!(controller(&mut native, &body), Some(None));
        } else {
            assert_eq!(controller(&mut native, &body), Some(None));
            prepare_saddle(&mut body);
            if kind == VANILLA_ENTITY_TYPE_PIG_ID {
                assert_eq!(controller(&mut native, &body), Some(None));
                for main in [true, false] {
                    native.world.stock_menu_slot_for_test(
                        ConnectionId(1),
                        36,
                        ItemStackSummary::empty(),
                    );
                    native.world.stock_menu_slot_for_test(
                        ConnectionId(1),
                        45,
                        ItemStackSummary::empty(),
                    );
                    player_item(&mut native, "minecraft:carrot_on_a_stick", main);
                    assert_eq!(controller(&mut native, &body), Some(Some(player)));
                }
            } else {
                assert_eq!(controller(&mut native, &body), Some(Some(player)));
            }
        }
    }
}

#[test]
fn shrieker_item_uses_actual_player_thrower_and_never_nonplayer_target() {
    let (mut native, player) = native_player();
    let mut store = store();
    let id = crate::entity::item::spawn_item_entity(
        &mut store,
        FEET,
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    )
    .unwrap();
    let mut body = store.detach(id).unwrap();
    assert_eq!(
        native.with_level(|level| try_get_player(&body, Some(&store), level)),
        Some(None)
    );
    if let EntityData::Item(data) = &mut body.data {
        data.set_thrower(Some(player.uuid));
    }
    assert_eq!(
        native.with_level(|level| try_get_player(&body, Some(&store), level)),
        Some(Some(player))
    );
    let owner = mob(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    if let EntityData::Item(data) = &mut body.data {
        data.set_thrower(Some(store.get(owner).unwrap().uuid()));
    }
    assert_eq!(
        native.with_level(|level| try_get_player(&body, Some(&store), level)),
        Some(None)
    );
}

#[test]
fn shrieker_step_contact_controller_and_projectile_raise_actual_player_sourced_shriek() {
    for living in [false, true] {
        let (mut native, player) = native_player();
        native.set(POS, state("minecraft:sculk_shrieker"));
        let mut store = store();
        let mut body = if living {
            let id = vehicle(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
            mount(&mut store, id, player.entity_id);
            let mut body = store.detach(id).unwrap();
            prepare_saddle(&mut body);
            player_item(&mut native, "minecraft:carrot_on_a_stick", false);
            body
        } else {
            arrow(&mut store, Some(player.uuid))
        };
        body.set_pos(Vec3d { y: 63.5, ..FEET });
        assert_eq!(
            native.with_level(|level| step_on_shrieker(&body, Some(&mut store), level, POS)),
            Some(())
        );
        let state = native.world.block_state(DIMENSION, POS).unwrap();
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_property(state, "shrieking"),
            Some("true")
        );
        assert!(native
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(|event| event.recorded.game_event
                == crate::level_event::VanillaGameEvent::SHRIEK
                && event.recorded.pos == POS
                && event.recorded.source_entity_id == Some(player.entity_id)));
    }
}

#[test]
fn shrieker_unsaddled_vehicle_has_no_controlling_player() {
    for kind in [VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_HORSE_ID] {
        let (mut native, player) = native_player();
        let mut store = store();
        let id = vehicle(&mut store, kind);
        mount(&mut store, id, player.entity_id);
        let body = store.detach(id).unwrap();
        assert_eq!(
            native.with_level(|level| try_get_player(&body, Some(&store), level)),
            Some(None)
        );
    }
}
