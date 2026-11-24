use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::level_event::{RecordedGameEvent, VanillaGameEvent};
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};

use crate::entity::Entity;

const GOLD_PLATE_Y: i32 = 63;
const FEET: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

const HALF_BLOCK: Vec3d = Vec3d {
    x: 0.5,
    y: -0.01,
    z: 0.0,
};

fn walking_store(entity_type_id: i32) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x575e_9eed, EntityIdCounter::new());
    let id = store
        .spawn_general_living(entity_type_id, FEET)
        .expect("the walking fixture is a general living type");
    (store, id)
}

fn plate(state: i32) -> NativeLevel {
    let mut level = NativeLevel::new();
    if state == state_id("minecraft:red_bed") {
        let foot = crate::fluid::with_property(state, "facing", "south").unwrap();
        for (z, part) in [(0, "foot"), (1, "head")] {
            assert!(level.world.set_block(
                DIMENSION,
                BlockPos { x: 0, y: 63, z },
                crate::fluid::with_property(foot, "part", part).unwrap(),
                2 | 16
            ));
        }
        return level;
    }
    for x in -4..=4 {
        for z in -4..=4 {
            assert!(level.world.set_block(
                DIMENSION,
                BlockPos {
                    x,
                    y: GOLD_PLATE_Y,
                    z
                },
                state,
                2 | 16
            ));
        }
    }
    level
}

fn gold_plate() -> NativeLevel {
    plate(state_id("minecraft:gold_block"))
}

fn game_events(level: &NativeLevel) -> Vec<RecordedGameEvent> {
    level
        .world
        .positioned_game_events_for_test(DIMENSION)
        .into_iter()
        .map(|event| event.recorded)
        .collect()
}

fn move_once(entity: &mut Entity, store: &mut EntityStore, level: &mut NativeLevel, delta: Vec3d) {
    level.with_level(|ctx| {
        crate::entity::move_entity(entity, store, ctx, crate::entity::MoverType::Self_, delta)
    });
}

#[test]
fn a_whole_block_of_travel_plays_one_step_sound() {
    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
        VANILLA_ENTITY_TYPE_RABBIT_ID,
    };
    // Zombie.java:368-369, Hoglin.java, CamelHusk.java override the block step.
    for (entity_type, source, sound, volume, pitch) in [
        (
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            SoundSource::Hostile,
            "minecraft:entity.zombie.step",
            0.15,
            1.0,
        ),
        (
            VANILLA_ENTITY_TYPE_HOGLIN_ID,
            SoundSource::Hostile,
            "minecraft:entity.hoglin.step",
            0.15,
            1.0,
        ),
        (
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            SoundSource::Neutral,
            "minecraft:entity.camel_husk.step",
            0.4,
            1.0,
        ),
        (
            VANILLA_ENTITY_TYPE_RABBIT_ID,
            SoundSource::Neutral,
            "minecraft:block.metal.step",
            0.15,
            1.5,
        ),
    ] {
        let (mut store, id) = walking_store(entity_type);
        let mut level = gold_plate();
        let mut entity = store.detach(id).unwrap();
        store.drain_lifecycle_events();

        for _ in 0..3 {
            move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
        }
        assert!(queued_sounds(&mut store).is_empty());
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);

        let mut expected = queued(sound, entity.position(), volume, pitch);
        expected.source = source;
        assert!(
            queued_sounds(&mut store).contains(&expected),
            "type {entity_type}"
        );
    }
}

#[test]
fn an_airborne_parrot_flaps_every_half_block_it_flies() {
    let (mut store, id) = walking_store(bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID);
    let mut level = NativeLevel::new();
    let mut entity = store.detach(id).unwrap();
    entity.set_pos(Vec3d { y: 100.0, ..FEET });
    store.drain_lifecycle_events();
    let flap =
        |entity: &Entity| queued("minecraft:entity.parrot.fly", entity.position(), 0.15, 1.0);

    let mut flaps = 0;
    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
        let expected = flap(&entity);
        flaps += queued_sounds(&mut store)
            .iter()
            .filter(|sound| **sound == expected)
            .count();
    }
    // Four half-block hops fly 1.2 blocks of `flyDist`: two flaps.
    assert_eq!(flaps, 2);
    let flap_events = game_events(&level)
        .iter()
        .filter(|event| {
            event.game_event == VanillaGameEvent::FLAP
                && event.source_entity_id == Some(entity.id())
        })
        .count();
    assert_eq!(flap_events, 2);
}

#[test]
fn walking_on_amethyst_chimes_at_most_once_a_second() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = plate(state_id("minecraft:amethyst_block"));
    let mut entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();
    let chime = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:block.amethyst_block.chime",
    )
    .unwrap();
    let chimes = |store: &mut EntityStore| {
        queued_sounds(store)
            .iter()
            .filter(|sound| sound.registry_id == chime)
            .count()
    };
    entity.tick_count = 20;
    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }
    assert_eq!(chimes(&mut store), 1);
    // Another step within the same twenty ticks stays silent.
    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }
    assert_eq!(chimes(&mut store), 0);
}

#[test]
fn travel_below_a_whole_block_plays_nothing() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();

    for _ in 0..3 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    assert!(queued_sounds(&mut store).is_empty());
    assert!(game_events(&level).is_empty());
}

#[test]
fn air_below_the_supporting_position_plays_nothing() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = NativeLevel::new();
    for x in -1..=4 {
        level.set(BlockPos { x, y: 63, z: 0 }, state_id("minecraft:air"));
    }
    let mut entity = store.detach(id).unwrap();
    entity.set_pose_id(bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID);
    store.drain_lifecycle_events();

    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    assert!(queued_sounds(&mut store).is_empty());
    assert!(game_events(&level).is_empty());
}

#[test]
fn an_item_move_emits_neither_sound_nor_event() {
    let mut store = EntityStore::new(0x1e3, EntityIdCounter::new());
    let id = spawn_item_entity(
        &mut store,
        FEET,
        plain_stack("minecraft:stone", 1).expect("stone is a built-in item"),
    )
    .expect("a freshly drawn UUID is free");
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    assert_eq!(
        entity.movement_emission(),
        crate::entity::MovementEmission::None
    );
    store.drain_lifecycle_events();

    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    assert!(queued_sounds(&mut store).is_empty());
    assert!(game_events(&level).is_empty());
}

#[test]
fn a_minecart_step_emits_an_event_without_a_sound() {
    let mut store = EntityStore::new(0x9a11, EntityIdCounter::new());
    let id = crate::entity::spawn_minecart(&mut store, FEET, 0.0).unwrap();
    assert_eq!(
        store.get(id).unwrap().entity_type_id(),
        VANILLA_ENTITY_TYPE_MINECART_ID
    );
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    assert_eq!(
        entity.movement_emission(),
        crate::entity::MovementEmission::Events
    );
    store.drain_lifecycle_events();

    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    assert!(queued_sounds(&mut store).is_empty());
    assert!(game_events(&level)
        .iter()
        .any(|actual| actual == &(step_event(&entity, state_id("minecraft:gold_block")))));
}

#[test]
fn a_silent_entity_walks_without_a_step_sound() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    store.get_mut(id).unwrap().set_silent(true);
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();

    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    assert!(queued_sounds(&mut store).is_empty());
}

#[test]
fn living_air_travel_walks_a_whole_block_and_plays_one_step_sound() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();
    let speed = entity
        .living()
        .unwrap()
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        as f32;
    entity.living_mut().unwrap().set_speed(speed);

    let forward = Vec3d {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };
    let mut sounds = Vec::new();
    for _ in 0..40 {
        level
            .with_level(|ctx| {
                crate::entity::living::travel_in_air(&mut entity, &mut store, ctx, forward)
            })
            .unwrap();
        sounds.extend(queued_sounds(&mut store));
        if !sounds.is_empty() {
            break;
        }
    }

    assert!(entity.position().z > FEET.z);

    let mut expected = queued("minecraft:entity.zombie.step", entity.position(), 0.15, 1.0);
    expected.source = SoundSource::Hostile;
    assert!(sounds.contains(&expected));
}

fn step_event(entity: &Entity, block_state: i32) -> RecordedGameEvent {
    RecordedGameEvent {
        game_event: VanillaGameEvent::STEP,
        pos: entity.block_position(),
        source_entity_id: Some(entity.id()),
        affected_state: Some(block_state),
    }
}

#[test]
fn a_whole_block_of_travel_raises_one_step_game_event() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    store.drain_lifecycle_events();

    for _ in 0..3 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }
    assert!(game_events(&level).is_empty());

    move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    assert!(game_events(&level)
        .iter()
        .any(|actual| actual == &(step_event(&entity, state_id("minecraft:gold_block")))));
}

#[test]
fn a_swimming_entity_in_water_raises_the_swim_game_event() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = gold_plate();
    level.set(BlockPos { x: 0, y: 64, z: 0 }, state_id("minecraft:water"));
    let mut entity = store.detach(id).unwrap();
    entity.set_swimming(true);
    entity.set_was_touching_water(true);
    store.drain_lifecycle_events();

    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    // Monster.java:57-59.
    let swim = bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
        "minecraft:entity.hostile.swim",
    );
    assert!(queued_sounds(&mut store)
        .iter()
        .any(|sound| Some(sound.registry_id) == swim));
    assert!(game_events(&level).iter().any(|actual| actual
        == &(RecordedGameEvent {
            game_event: VanillaGameEvent::SWIM,
            pos: entity.block_position(),
            source_entity_id: Some(entity.id()),
            affected_state: None,
        })));
}

#[test]
fn a_swimming_entity_out_of_water_raises_nothing() {
    let (mut store, id) = walking_store(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let mut level = gold_plate();
    let mut entity = store.detach(id).unwrap();
    entity.set_swimming(true);
    store.drain_lifecycle_events();

    for _ in 0..4 {
        move_once(&mut entity, &mut store, &mut level, HALF_BLOCK);
    }

    assert!(queued_sounds(&mut store).is_empty());
    assert!(game_events(&level).is_empty());
}

mod fall;

mod ground;

mod velocity;
