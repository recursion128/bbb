use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::item_types::{VANILLA_ITEM_CARROT_ON_A_STICK_ID, VANILLA_ITEM_WHEAT_ID};
use uuid::Uuid;

use super::*;
use crate::connection::ConnectionId;
use crate::entity::ai::mob::spec::{mob_spec, GoalSpec};
use crate::entity::ai::mob::Sensing;
use crate::entity::general_living::GeneralLivingData;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::EntityData;
use crate::random::GameRandom;
use crate::tests::mailbox;
use bbb_protocol::packets::ItemStackSummary;

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

fn catalogued_tempt_spec(entity_type_id: i32) -> TemptSpec {
    mob_spec(entity_type_id)
        .unwrap()
        .goals
        .iter()
        .find_map(|(_, goal)| match *goal {
            GoalSpec::Tempt {
                speed,
                items,
                scare,
            } => TemptSpec::new(items, scare, speed, entity_type_id),
            _ => None,
        })
        .unwrap()
}

#[test]
fn nearest_tempting_player_tracks_held_food_and_live_position() {
    let spec = catalogued_tempt_spec(VANILLA_ENTITY_TYPE_COW_ID);
    let mut native = NativeLevel::new();
    let mut players = Vec::new();
    let (sender, receiver) = mailbox();
    for (connection, x, main, off) in [
        (1, 1.5, Some(VANILLA_ITEM_WHEAT_ID), None),
        (2, 2.5, Some(VANILLA_ITEM_CARROT_ON_A_STICK_ID), None),
        (3, 3.5, None, None),
        (4, 4.5, Some(VANILLA_ITEM_WHEAT_ID), None),
        (5, 5.5, None, Some(VANILLA_ITEM_WHEAT_ID)),
        (6, 40.5, Some(VANILLA_ITEM_WHEAT_ID), None),
    ] {
        crate::tests::join_loaded(
            &sender,
            &receiver,
            &mut native.world,
            connection,
            &format!("player{connection}"),
            u128::from(connection),
        );
        let id = ConnectionId(connection);
        native.world.set_player_position_for_test(id, position(x));
        for (slot, item) in [(36, main), (45, off)] {
            if let Some(item_id) = item {
                native.world.stock_menu_slot_for_test(
                    id,
                    slot,
                    ItemStackSummary {
                        item_id: Some(item_id),
                        count: 1,
                        component_patch: Default::default(),
                    },
                );
            }
        }
        players.push(native.world.player_entity_id_for_test(id));
    }
    let abilities = native.world.player_abilities_for_test(ConnectionId(4));
    native.world.set_player_mode_and_abilities_for_test(
        ConnectionId(4),
        bbb_protocol::game_types::GameType::Spectator,
        abilities,
    );
    let ids = EntityIdCounter::new();
    for _ in 0..*players.iter().max().unwrap() {
        ids.next_id();
    }
    let mut store = EntityStore::new(1, ids);
    let cow = store
        .restore(
            Uuid::from_u128(9),
            VANILLA_ENTITY_TYPE_COW_ID,
            position(0.5),
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
        )
        .unwrap();
    let body = store.get(cow).unwrap();
    let mut context = MobAiContext::from_living_body(
        cow,
        1,
        body.position(),
        body.bounding_box(),
        body.position().y + f64::from(body.eye_height()),
        body.x_rot(),
        body.y_rot(),
        body.on_ground,
        false,
        false,
        body.living().unwrap(),
        GameRandom::with_seed(0),
    );
    let sensing = Sensing::default();
    let mut goal = TemptGoal::new(spec);
    native.with_level(|level| {
        let mut targets = TargetView::new(body, &store, level, &sensing);
        assert!(goal.can_use(&mut context, &mut targets));
        assert_eq!(goal.target_id, Some(players[0]));
        goal.start(&mut context, &mut targets);
    });
    native.world.stock_menu_slot_for_test(
        ConnectionId(1),
        36,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_CARROT_ON_A_STICK_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    native.with_level(|level| {
        let mut targets = TargetView::new(body, &store, level, &sensing);
        assert!(goal.can_continue(&mut context, &mut targets));
        assert_eq!(goal.target_id, Some(players[4]));
    });
    native
        .world
        .set_player_position_for_test(ConnectionId(5), position(6.5));
    native.with_level(|level| {
        let mut targets = TargetView::new(body, &store, level, &sensing);
        goal.tick(&mut context, &mut targets);
        assert_eq!(context.controls.look_target.unwrap().x, 6.5);
    });
    native.world.stock_menu_slot_for_test(
        ConnectionId(5),
        45,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_CARROT_ON_A_STICK_ID),
            count: 1,
            component_patch: Default::default(),
        },
    );
    native.with_level(|level| {
        let mut targets = TargetView::new(body, &store, level, &sensing);
        assert!(!goal.can_continue(&mut context, &mut targets));
        assert_eq!(goal.target_id, None);
    });
}

/// How far a fresh mob walks toward a player holding `item` in 80 ticks.
fn walked_toward_player(entity_type_id: i32, item_id: i32) -> f64 {
    let mut native = NativeLevel::new();
    native
        .world
        .load_entity_chunk_for_test(crate::entity::tests::native_level::DIMENSION, (0, 0))
        .unwrap();
    let (sender, receiver) = mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "player1", 1);
    let player = ConnectionId(1);
    native.world.set_player_position_for_test(
        player,
        Vec3d {
            x: 9.5,
            y: 64.0,
            z: 8.5,
        },
    );
    native.world.stock_menu_slot_for_test(
        player,
        36,
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: Default::default(),
        },
    );
    let start = Vec3d {
        x: 2.5,
        y: 64.0,
        z: 8.5,
    };
    let ids = EntityIdCounter::new();
    for _ in 0..native.world.player_entity_id_for_test(player) {
        ids.next_id();
    }
    let mut store = EntityStore::new(3, ids);
    let id = store.spawn_general_living(entity_type_id, start).unwrap();
    for _ in 0..80 {
        native.with_level(|level| {
            crate::entity::tick_one_entity(&mut store, level, id);
        });
    }
    store.get(id).unwrap().position().x - start.x
}

#[test]
fn cats_ocelots_and_turtles_follow_their_food() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    };
    use bbb_protocol::item_types::{VANILLA_ITEM_COD_ID, VANILLA_ITEM_SEAGRASS_ID};
    for (kind, food) in [
        (VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ITEM_COD_ID),
        (VANILLA_ENTITY_TYPE_OCELOT_ID, VANILLA_ITEM_COD_ID),
        (VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ITEM_SEAGRASS_ID),
    ] {
        let walked = walked_toward_player(kind, food);
        assert!(walked > 3.0, "{kind}: {walked}");
    }
}
