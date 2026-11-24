use super::*;
use crate::connection::ConnectionId;
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::{EntityIdCounter, ExternalPlayerCanRide};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::entity_types::*;
use bbb_protocol::nbt_summary::json_value_to_nbt_summary;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::Vec3d;

fn fixture(kind: i32) -> (EntityStore, i32) {
    let ids = EntityIdCounter::new();
    ids.next_id();
    let mut store = EntityStore::new(9, ids);
    let id = if kind == VANILLA_ENTITY_TYPE_OAK_BOAT_ID {
        crate::entity::spawn_boat(&mut store, kind, Vec3d::default(), 0.0).unwrap()
    } else {
        store.spawn_general_living(kind, Vec3d::default()).unwrap()
    };
    (store, id)
}

fn attach(store: &mut EntityStore, id: i32, passenger: i32) {
    assert!(store.attach_test_player(passenger, id, ExternalPlayerCanRide::Allowed));
}

fn equip(store: &mut EntityStore, id: i32, resource: &str, slot: EquipmentSlot) {
    store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(slot, crate::entity::item::plain_stack(resource, 1));
}

fn native_player() -> (NativeLevel, PlayerIdentity) {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 99);
    let player = PlayerIdentity {
        uuid: Uuid::from_u128(99),
        entity_id: native.world.player_entity_id_for_test(ConnectionId(1)),
    };
    (native, player)
}

fn query(
    entity: &Entity,
    store: Option<&EntityStore>,
    native: &mut NativeLevel,
) -> Option<Option<PlayerIdentity>> {
    native.with_level(|level| {
        direct_controlling_player_with_registry_tags(
            entity,
            store,
            &level.capture_registry_tags(),
            |id| level.player_by_entity_id(id),
        )
    })
}

fn held(native: &mut NativeLevel, resource: &str, main: bool) {
    for slot in [36, 45] {
        native
            .world
            .stock_menu_slot_for_test(ConnectionId(1), slot, ItemStackSummary::empty());
    }
    native.world.stock_menu_slot_for_test(
        ConnectionId(1),
        if main { 36 } else { 45 },
        ItemStackSummary {
            item_id: Some(vanilla_item_type_for_resource_id(resource).unwrap().id),
            count: 1,
            component_patch: Default::default(),
        },
    );
}

fn non_controlling_tag(native: &mut NativeLevel, members: &[&str]) -> EntityRegistryTagView {
    native.install_tags(RegistryTagSnapshot::new([(
        "minecraft:entity_type".to_owned(),
        "minecraft:non_controlling_rider".to_owned(),
        members.iter().map(|member| (*member).to_owned()).collect(),
    )]));
    native.with_level(|level| level.capture_registry_tags())
}

#[test]
fn player_control_public_query_distinguishes_direct_player_from_mob_and_owner() {
    let (mut native, player) = native_player();
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d::default())
        .unwrap();
    assert!(store.link_existing_passenger(id, cow,));
    attach(&mut store, cow, player.entity_id);
    let body = store.detach(id).unwrap();
    assert_eq!(
        direct_controlling_player_with_registry_tags(
            &body,
            Some(&store),
            &EntityRegistryTagView::Builtin26_1,
            |_| None
        ),
        Some(None)
    );
    assert_eq!(
        direct_controlling_living_with_registry_tags(
            &body,
            Some(&store),
            &EntityRegistryTagView::Builtin26_1,
            |_| None
        ),
        Some(Some(DirectLivingController::Stored(cow)))
    );
    let arrow = crate::entity::arrow::spawn_arrow(
        &mut store,
        Vec3d::default(),
        Vec3d::default(),
        crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
    )
    .unwrap();
    let mut arrow = store.detach(arrow).unwrap();
    let EntityData::Arrow(data) = &mut arrow.data else {
        unreachable!()
    };
    data.projectile.set_owner(Some(Uuid::from_u128(99)));
    assert_eq!(query(&arrow, Some(&store), &mut native), Some(None));
}

#[test]
fn player_control_external_identity_accepts_actual_dead_spectator() {
    let (mut native, player) = native_player();
    let abilities = native.world.player_abilities_for_test(ConnectionId(1));
    native.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        bbb_protocol::game_types::GameType::Spectator,
        abilities,
    );
    native.world.set_player_dead_for_test(ConnectionId(1));
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    attach(&mut store, id, player.entity_id);
    let body = store.detach(id).unwrap();
    assert_eq!(query(&body, Some(&store), &mut native), Some(Some(player)));
}

#[test]
fn player_control_uses_actual_main_and_offhand_items_and_happy_still_state() {
    for (kind, control) in [
        (VANILLA_ENTITY_TYPE_PIG_ID, "minecraft:carrot_on_a_stick"),
        (
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            "minecraft:warped_fungus_on_a_stick",
        ),
    ] {
        let (mut native, player) = native_player();
        let (mut store, id) = fixture(kind);
        attach(&mut store, id, player.entity_id);
        equip(&mut store, id, "minecraft:saddle", EquipmentSlot::Saddle);
        let body = store.detach(id).unwrap();
        assert_eq!(query(&body, Some(&store), &mut native), Some(None));
        for main in [true, false] {
            held(&mut native, control, main);
            assert_eq!(query(&body, Some(&store), &mut native), Some(Some(player)));
        }
    }
    let (mut native, player) = native_player();
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
    attach(&mut store, id, player.entity_id);
    equip(
        &mut store,
        id,
        "minecraft:white_harness",
        EquipmentSlot::Body,
    );
    let mut body = store.detach(id).unwrap();
    for (timer, controlled) in [(10, false), (0, true)] {
        crate::entity::happy_ghast::set_server_still_timeout(&mut body, timer);
        assert_eq!(
            query(&body, Some(&store), &mut native).unwrap().is_some(),
            controlled
        );
    }
}

#[test]
fn player_control_loaded_equippable_tag_is_shared_with_valid_equipment() {
    let (mut native, player) = native_player();
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_HORSE_ID);
    attach(&mut store, id, player.entity_id);
    let stack = bbb_server_types::item_stack::RuntimeItemStack::try_read(&json_value_to_nbt_summary(&serde_json::json!({
        "id": "minecraft:stone", "count": 1,
        "components": {"minecraft:equippable": {"slot": "saddle", "allowed_entities": "#example:mount"}}
    })).unwrap(), &crate::entity::test_items::BuiltinItems).unwrap();
    store
        .get_mut(id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::Saddle, Some(stack));
    let body = store.detach(id).unwrap();
    for (member, expected) in [("minecraft:horse", true), ("minecraft:pig", false)] {
        native.install_tags(RegistryTagSnapshot::new([(
            "minecraft:entity_type".to_owned(),
            "example:mount".to_owned(),
            vec![member.to_owned()],
        )]));
        assert_eq!(
            query(&body, Some(&store), &mut native).unwrap().is_some(),
            expected
        );
    }
}

#[test]
fn player_control_full_living_fallback_shares_no_ai_and_loaded_mob_tag_semantics() {
    for kind in [
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
    ] {
        let mut native = NativeLevel::new();
        let (mut store, id) = fixture(kind);
        let rider = store
            .spawn_general_living(VANILLA_ENTITY_TYPE_PILLAGER_ID, Vec3d::default())
            .unwrap();
        assert!(store.link_existing_passenger_forced(id, rider));
        let body = store.get(id).unwrap();
        assert_eq!(
            direct_controlling_living_with_registry_tags(
                body,
                Some(&store),
                &non_controlling_tag(&mut native, &[]),
                |_| None
            ),
            Some(Some(DirectLivingController::Stored(rider)))
        );
        assert_eq!(
            direct_controlling_living_with_registry_tags(
                body,
                Some(&store),
                &non_controlling_tag(&mut native, &["minecraft:pillager"]),
                |_| None
            ),
            Some(None)
        );
        assert_eq!(
            query(body, Some(&store), &mut NativeLevel::new()),
            Some(None)
        );
        let EntityData::GeneralLiving(data) = &mut store.get_mut(id).unwrap().data else {
            unreachable!()
        };
        data.set_no_ai(true);
        assert_eq!(
            direct_controlling_living_with_registry_tags(
                store.get(id).unwrap(),
                Some(&store),
                &EntityRegistryTagView::Builtin26_1,
                |_| None
            ),
            Some(None)
        );
    }
}

#[test]
fn player_control_boat_accepts_non_mob_living_without_mob_tag_lookup() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    let stand = crate::entity::spawn_armor_stand(&mut store, Vec3d::default()).unwrap();
    assert!(store.link_existing_passenger_forced(id, stand));
    let body = store.detach(id).unwrap();
    assert_eq!(
        direct_controlling_living_with_registry_tags(
            &body,
            Some(&store),
            &EntityRegistryTagView::Builtin26_1,
            |_| None,
        ),
        Some(Some(DirectLivingController::Stored(stand)))
    );
    assert_eq!(
        query(&body, Some(&store), &mut NativeLevel::new()),
        Some(None)
    );
}

#[test]
fn player_control_happy_still_falls_back_to_mob() {
    let (mut store, id) = fixture(VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID);
    let rider = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PILLAGER_ID, Vec3d::default())
        .unwrap();
    assert!(store.link_existing_passenger_forced(id, rider));
    equip(
        &mut store,
        id,
        "minecraft:white_harness",
        EquipmentSlot::Body,
    );
    let mut body = store.detach(id).unwrap();
    crate::entity::happy_ghast::set_server_still_timeout(&mut body, 10);
    assert_eq!(
        direct_controlling_living_with_registry_tags(
            &body,
            Some(&store),
            &EntityRegistryTagView::Builtin26_1,
            |_| None,
        ),
        Some(Some(DirectLivingController::Stored(rider)))
    );
    assert_eq!(
        query(&body, Some(&store), &mut NativeLevel::new()),
        Some(None)
    );
    let EntityData::GeneralLiving(data) = &mut body.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    assert_eq!(
        direct_controlling_living_with_registry_tags(
            &body,
            Some(&store),
            &EntityRegistryTagView::Builtin26_1,
            |_| None,
        ),
        Some(None)
    );
}

#[test]
fn player_may_build_reads_the_actual_sessions_game_mode_ability() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "builder", 1000);
    let uuid = native
        .world
        .nearby_players_for_test(crate::entity::tests::native_level::DIMENSION)[0]
        .uuid;
    for (mode, may_build, instabuild) in [
        ("adventure", false, false),
        ("creative", true, true),
        ("survival", true, false),
    ] {
        crate::tests::console_command(
            &sender,
            &receiver,
            &mut native.world,
            1,
            &format!("gamemode {mode} builder"),
        );
        native.with_level(|level| {
            assert_eq!(level.player(uuid).unwrap().instabuild, instabuild);
            assert_eq!(level.player_may_build(uuid), may_build);
        });
    }
}
