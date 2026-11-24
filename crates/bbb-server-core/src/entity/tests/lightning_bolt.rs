use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::world::WorldEvent;
use bbb_protocol::game_types::GameType;
use bbb_protocol::packets::PlayerAbilities;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID;
use bbb_protocol::packets::Vec3d;

use crate::entity::save::write_entity_chunk;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::{spawn_lightning_bolt, tick_entities, EntityData, LightningBoltData};
use crate::level_event::VanillaGameEvent;

const STRIKE_POS: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn bolt(store: &mut EntityStore) -> i32 {
    let id = spawn_lightning_bolt(store, STRIKE_POS, false, None).expect("a fresh UUID");
    store.drain_lifecycle_events();
    id
}

fn bolt_with_flashes(store: &mut EntityStore, flashes: i32) -> i32 {
    let uuid = store.next_uuid();
    let id = store
        .restore(
            uuid,
            VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
            STRIKE_POS,
            EntityData::LightningBolt(LightningBoltData::new(0, flashes)),
        )
        .expect("a fresh UUID");
    store.drain_lifecycle_events();
    id
}

fn bolt_data(store: &EntityStore, id: i32) -> LightningBoltData {
    match store.get(id).expect("entity is live").data() {
        EntityData::LightningBolt(data) => data.clone(),
        other => panic!("expected a lightning bolt, found {other:?}"),
    }
}

#[test]
fn a_spawned_bolt_starts_at_life_two_with_drawn_seed_and_flash_count() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = bolt(&mut store);
    let entity = store.get(id).unwrap();
    assert_eq!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID
    );
    let data = bolt_data(&store, id);
    assert_eq!(data.life(), 2);
    assert!((1..=3).contains(&data.flashes()));
    assert!(!data.visual_only());
    assert_eq!(data.blocks_set_on_fire(), 0);
}

#[test]
fn a_bolt_raises_the_lightning_strike_game_event() {
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    bolt_with_flashes(&mut store, 1);
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::LIGHTNING_STRIKE));
}

#[test]
fn a_bolt_thunder_hits_a_player_in_range_with_the_lightning_source() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "victim", 1);
    level
        .world
        .set_player_position_for_test(ConnectionId(1), STRIKE_POS);
    level.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let before = level.world.player_health_for_test(ConnectionId(1));
    level.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
        STRIKE_POS,
        EntityData::LightningBolt(LightningBoltData::new(0, 1)),
    );
    let output = level.world.tick(receiver.drain());
    assert_eq!(
        level.world.player_health_for_test(ConnectionId(1)),
        before - 5.0
    );
    let kind = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:lightning_bolt")
        .unwrap()
        .registry_id;
    assert!(output.events.iter().any(|event|matches!(event,WorldEvent::Send { packet: ClientboundPlayPacket::DamageEvent(hurt),.. } if hurt.source_type_id==kind && hurt.source_cause_id == -1 && hurt.source_direct_id == -1)));
}

#[test]
fn a_visual_only_bolt_never_hits_anyone_but_still_flashes_and_removes() {
    let mut level = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut level.world, 1, "victim", 1);
    level.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            y: 64.0,
            ..STRIKE_POS
        },
    );
    level.world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
    let before = level.world.player_health_for_test(ConnectionId(1));
    let mut data = LightningBoltData::new(0, 1);
    data.set_visual_only(true);
    let id = level
        .world
        .spawn_stored_entity_for_test(
            DIMENSION,
            VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
            STRIKE_POS,
            EntityData::LightningBolt(data),
        )
        .0;
    for _ in 0..200 {
        level.world.tick(receiver.drain());
        if level.world.stored_entity_for_test(DIMENSION, id).is_none() {
            break;
        }
    }
    assert!(level.world.stored_entity_for_test(DIMENSION, id).is_none());
    assert_eq!(level.world.player_health_for_test(ConnectionId(1)), before);
    assert!(level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.recorded.game_event == VanillaGameEvent::LIGHTNING_STRIKE));
}

#[test]
fn a_bolt_removes_itself_once_its_flashes_run_out() {
    let mut store = EntityStore::new(5, EntityIdCounter::new());
    let id = bolt_with_flashes(&mut store, 1);
    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));
    assert!(store.get(id).is_some(), "still alive at life 1");
    for _ in 0..200 {
        level.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).is_none() {
            break;
        }
    }
    assert!(store.get(id).is_none(), "the bolt removed itself");
}

/// `Entity.thunderHit` burns an item (`ItemEntity` health 5 against the
/// strike's 5); `ArmorStand.thunderHit` is empty (`ArmorStand.java:602-603`).
#[test]
fn a_bolt_burns_a_stored_item_and_leaves_an_armor_stand_untouched() {
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let item =
        crate::entity::spawn_item_entity(&mut store, STRIKE_POS, stack_of("minecraft:dirt", 1))
            .expect("a fresh UUID");
    let stand = crate::entity::spawn_armor_stand(&mut store, STRIKE_POS).expect("a fresh UUID");
    bolt_with_flashes(&mut store, 1);
    store.drain_lifecycle_events();
    let stand_health = store.get(stand).unwrap().living().unwrap().health();

    let mut level = NativeLevel::new();
    level.with_level(|level| tick_entities(&mut store, level, true));

    assert!(store.get(item).is_none_or(|item| !item.is_alive()));
    assert_eq!(
        store.get(stand).unwrap().living().unwrap().health(),
        stand_health
    );
}

#[test]
fn a_lightning_bolt_is_never_written_to_a_chunk() {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let _ = bolt(&mut store);
    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert!(document.is_empty());
}

fn copper_strike(struck: &str) -> (NativeLevel, BlockPos) {
    let mut store = EntityStore::new(7, EntityIdCounter::new());
    let uuid = store.next_uuid();
    store
        .restore(
            uuid,
            VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
            Vec3d {
                x: 8.5,
                y: 65.0,
                z: 8.5,
            },
            EntityData::LightningBolt(LightningBoltData::new(0, 1)),
        )
        .unwrap();
    let struck_pos = BlockPos { x: 8, y: 64, z: 8 };
    let mut level = NativeLevel::new();
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                level.set(
                    BlockPos {
                        x: 8 + dx,
                        y: 64 + dy,
                        z: 8 + dz,
                    },
                    state_id("minecraft:weathered_copper"),
                );
            }
        }
    }
    level.set(struck_pos, state_id(struck));
    level.with_level(|level| tick_entities(&mut store, level, true));
    (level, struck_pos)
}

fn cleaned_neighbours(level: &mut NativeLevel, struck: BlockPos) -> usize {
    let weathered = state_id("minecraft:weathered_copper");
    let younger = [
        state_id("minecraft:exposed_copper"),
        state_id("minecraft:copper_block"),
    ];
    let events = level.world.drain_level_events_for_test(DIMENSION);
    let mut cleaned = 0;
    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let pos = BlockPos {
                    x: struck.x + dx,
                    y: struck.y + dy,
                    z: struck.z + dz,
                };
                if pos == struck {
                    continue;
                }
                let state = level.world.block_state(DIMENSION, pos).unwrap();
                assert!(state == weathered || younger.contains(&state));
                if state != weathered {
                    cleaned += 1;
                    assert!(events.iter().any(|event| event.event_type == 3002
                        && event.pos == pos
                        && event.data == -1));
                }
            }
        }
    }
    cleaned
}

#[test]
fn a_strike_on_copper_scrapes_it_bare_and_cleans_nearby_copper() {
    let (mut level, struck) = copper_strike("minecraft:oxidized_copper");
    assert_eq!(
        level.world.block_state(DIMENSION, struck),
        Some(state_id("minecraft:copper_block"))
    );
    assert!(cleaned_neighbours(&mut level, struck) > 0);
}

#[test]
fn a_strike_on_waxed_copper_keeps_the_wax_but_still_cleans_nearby_copper() {
    let (mut level, struck) = copper_strike("minecraft:waxed_oxidized_copper");
    assert_eq!(
        level.world.block_state(DIMENSION, struck),
        Some(state_id("minecraft:waxed_oxidized_copper"))
    );
    assert!(cleaned_neighbours(&mut level, struck) > 0);
}

#[test]
fn a_strike_on_plain_stone_leaves_nearby_copper_alone() {
    let (mut level, struck) = copper_strike("minecraft:stone");
    assert_eq!(cleaned_neighbours(&mut level, struck), 0);
}

fn copper_golem_weather(store: &EntityStore, id: i32) -> (Option<String>, Option<i64>) {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        panic!("a copper golem is living");
    };
    let state = data.copper_golem_state().expect("copper golem state");
    (
        Some(
            crate::entity::copper_golem::CopperGolemState::weather_state_name(state.weather_state)
                .to_owned(),
        ),
        Some(state.next_weather_age),
    )
}

#[test]
fn one_bolt_scrapes_a_waxed_oxidized_copper_golem_back_one_stage() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID;
    let mut store = EntityStore::new(8, EntityIdCounter::new());
    let mut data =
        crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID).unwrap();
    assert!(crate::entity::copper_golem::set_weather_state(
        &mut data, "oxidized"
    ));
    data.copper_golem_state_mut().unwrap().next_weather_age = -2;
    let golem = store.spawn(
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
        STRIKE_POS,
        EntityData::GeneralLiving(data),
    );
    bolt_with_flashes(&mut store, 3);
    let mut level = NativeLevel::new();
    for _ in 0..20 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let (state, age) = copper_golem_weather(&store, golem);
    assert_eq!(state.as_deref(), Some("weathered"));
    assert_ne!(age, Some(-2), "the strike unwaxes the golem");
}
