use super::native_level::NativeLevel;
use super::*;

use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID, VANILLA_ARROW_EFFECT_COLOR_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARROW_ID;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::NbtSummaryValue;

use crate::entity::item::plain_stack;
use crate::entity::portal::PortalKind;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::store::{EntityIdCounter, EntityLifecycleEvent, EntityStore};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{
    spawn_arrow, tick_entities, tick_one_entity, ArrowData, ArrowPickup, EntityData,
    EntityTickOutcome,
};

fn arrow_stack() -> RuntimeItemStack {
    plain_stack("minecraft:arrow", 1).expect("arrow is a built-in stack")
}

fn flying_arrow() -> (EntityStore, NativeLevel, i32) {
    let mut store = EntityStore::new(11, EntityIdCounter::new());
    let level = NativeLevel::new();
    let id = spawn_arrow(
        &mut store,
        Vec3d {
            x: 0.5,
            y: 68.0,
            z: 0.5,
        },
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        },
        arrow_stack(),
    )
    .expect("a fresh UUID");
    store.drain_lifecycle_events();
    (store, level, id)
}

fn arrow_data(store: &EntityStore, id: i32) -> ArrowData {
    match store.get(id).expect("entity is live").data() {
        EntityData::Arrow(data) => data.clone(),
        other => panic!("expected an arrow, found {other:?}"),
    }
}

#[test]
fn a_spawned_arrow_carries_motion_and_pickup_defaults() {
    let (store, _, id) = flying_arrow();
    let entity = store.get(id).unwrap();
    assert_eq!(entity.entity_type_id(), VANILLA_ENTITY_TYPE_ARROW_ID);
    assert_eq!(
        entity.delta_movement(),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0
        }
    );
    assert_eq!(arrow_data(&store, id).pickup(), ArrowPickup::Disallowed);
    assert_eq!(
        arrow_data(&store, id).damage(),
        crate::entity::arrow::DEFAULT_DAMAGE
    );
    assert!(non_default_synched_values(entity).is_empty());
}

#[test]
fn replacing_an_arrow_item_publishes_its_potion_colour() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;
    use bbb_protocol::packets::EntityDataValueKind;

    let (mut store, mut level, id) = flying_arrow();
    let poison =
        bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id("minecraft:poison")
            .unwrap();
    for (contents, expected) in [
        ("{}", -1),
        ("{custom_name:'named'}", -13_083_194),
        ("{custom_color:1193046}", 0x0012_3456),
        (
            "{potion:'minecraft:poison',custom_effects:[{id:'minecraft:speed',amplifier:3,duration:200,show_particles:0b}]}",
            (0xff00_0000 | poison.color) as i32,
        ),
        (
            "{custom_effects:[{id:'minecraft:poison',duration:200,show_particles:0b}]}",
            -13_083_194,
        ),
    ] {
        let contents = bbb_protocol::nbt_summary::parse_snbt_compound(contents).unwrap();
        let item = arrow_stack()
            .with_component_nbt(
                VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
                Some(&contents),
                &bbb_registry_codec::UnavailableHolderResolver,
            )
            .unwrap();
        assert!(level
            .with_level(|level| crate::entity::item_slots::write(&mut store, level, id, 0, item))
            .unwrap()
            .0);
        assert_eq!(arrow_data(&store, id).effect_color(), expected);
        let colour = non_default_synched_values(store.get(id).unwrap())
            .into_iter()
            .find_map(|row| match row.value {
                EntityDataValueKind::Int(value) if row.data_id == VANILLA_ARROW_EFFECT_COLOR_DATA_ID => Some(value),
                _ => None,
            });
        assert_eq!(colour, (expected != -1).then_some(expected));
    }
}

fn land_arrow(store: &mut EntityStore, level: &mut NativeLevel, id: i32) {
    for _ in 0..200 {
        level.with_level(|level| tick_entities(store, level, true));
        if store
            .get(id)
            .and_then(|entity| crate::entity::abstract_arrow::state(entity.data()))
            .is_some_and(|state| state.in_ground())
        {
            return;
        }
    }
    panic!("arrow never landed");
}

#[test]
fn a_stuck_tipped_arrow_loses_its_potion_after_thirty_seconds() {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID;

    let (mut store, mut level, id) = flying_arrow();
    let contents =
        bbb_protocol::nbt_summary::parse_snbt_compound("{potion:'minecraft:poison'}").unwrap();
    let item = arrow_stack()
        .with_component_nbt(
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            Some(&contents),
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .unwrap();
    level
        .with_level(|level| crate::entity::item_slots::write(&mut store, level, id, 0, item))
        .unwrap();
    land_arrow(&mut store, &mut level, id);
    arrow_data_mut(&mut store, id).restore_in_ground_time(599);

    level.with_level(|level| tick_one_entity(&mut store, level, id));

    let data = arrow_data(&store, id);
    assert_eq!(data.effect_color(), -1);
    assert_eq!(data.item(), &arrow_stack());
}

#[test]
fn an_arrow_falls_and_sticks_in_the_ground() {
    let (mut store, mut level, id) = flying_arrow();
    land_arrow(&mut store, &mut level, id);
    let entity = store.get(id).unwrap();
    let data = arrow_data(&store, id);
    assert!(data.in_ground());
    assert_eq!(data.shake_time(), 7);
    assert_eq!(entity.delta_movement(), Vec3d::default());
    let synched = non_default_synched_values(entity);
    assert!(
        synched
            .iter()
            .any(|row| row.data_id == VANILLA_ABSTRACT_ARROW_IN_GROUND_DATA_ID),
        "in-ground is published"
    );
}

#[test]
fn a_flying_arrow_contacts_a_portal() {
    let mut store = EntityStore::new(12, EntityIdCounter::new());
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let movement = Vec3d {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    };
    let id = spawn_arrow(&mut store, start, movement, arrow_stack()).unwrap();
    let entry = BlockPos { x: 1, y: 68, z: 0 };
    let mut level = NativeLevel::new();
    level.set(entry, state_id("minecraft:nether_portal"));

    let EntityTickOutcome::Portal(pending) =
        level.with_level(|level| tick_one_entity(&mut store, level, id))
    else {
        panic!("the arrow contacts the portal");
    };

    assert_eq!(pending.requester_entity_id(), id);
    assert_eq!(pending.request().kind, PortalKind::Nether);
    assert_eq!(pending.request().entry, entry);
}

#[test]
fn a_no_physics_arrow_crosses_a_portal_without_entering_it() {
    let mut store = EntityStore::new(13, EntityIdCounter::new());
    let start = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let id = spawn_arrow(
        &mut store,
        start,
        Vec3d {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
        arrow_stack(),
    )
    .unwrap();
    arrow_data_mut(&mut store, id).set_no_physics(true);
    let mut level = NativeLevel::new();
    level.set(
        BlockPos { x: 1, y: 68, z: 0 },
        state_id("minecraft:nether_portal"),
    );

    level.with_level(|level| tick_one_entity(&mut store, level, id));

    let entity = store.get(id).unwrap();
    assert_eq!(entity.position().x, 2.5);
    assert!(entity.portal_process().is_none());
}

#[test]
fn an_in_ground_arrow_contacts_a_portal() {
    let mut store = EntityStore::new(14, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let id = spawn_arrow(&mut store, position, Vec3d::default(), arrow_stack()).unwrap();
    arrow_data_mut(&mut store, id).set_in_ground(true);
    let entry = BlockPos { x: 0, y: 68, z: 0 };
    let mut level = NativeLevel::new();
    level.set(entry, state_id("minecraft:nether_portal"));

    level.with_level(|level| tick_one_entity(&mut store, level, id));

    let processor = store.get(id).unwrap().portal_process().unwrap();
    assert_eq!(processor.kind(), PortalKind::Nether);
    assert_eq!(processor.entry_position(), entry);
}

#[test]
fn a_stuck_arrow_despawns_after_twelve_hundred_ticks() {
    let (mut store, mut level, id) = flying_arrow();
    land_arrow(&mut store, &mut level, id);
    arrow_data_mut(&mut store, id).set_pickup(ArrowPickup::Allowed);
    for _ in 0..1200 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(id).is_none());
    let removed = store
        .drain_lifecycle_events()
        .into_iter()
        .any(|event| matches!(event, EntityLifecycleEvent::Removed { .. }));
    assert!(removed);
}

#[test]
fn an_arrow_round_trips_through_the_entity_chunk_save_form() {
    let (mut store, mut level, id) = flying_arrow();
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        panic!("the arrow command data is a compound");
    };
    entries
        .iter_mut()
        .find(|entry| entry.name == "SoundEvent")
        .unwrap()
        .value = NbtSummaryValue::String("block.bell.use".into());
    store.load_command_data(id, &entries).unwrap();
    {
        let entity = store.get_mut(id).unwrap();
        entity.set_rot(45.0, -15.0);
        entity.set_delta_movement(Vec3d {
            x: 0.2,
            y: -0.3,
            z: 0.1,
        });
    }
    {
        let data = arrow_data_mut(&mut store, id);
        data.set_pickup(ArrowPickup::Allowed);
        data.set_crit(true);
        data.set_effect_color(0x00ff_aabb);
    }
    for _ in 0..2 {
        level.with_level(|level| tick_entities(&mut store, level, true));
    }
    let before = store.get(id).unwrap().clone();
    let before_data = arrow_data(&store, id);
    assert_eq!(
        before_data.sound_event_resource_id(),
        "minecraft:block.bell.use"
    );
    assert!(
        non_default_synched_values(&before)
            .iter()
            .any(|row| row.data_id == VANILLA_ARROW_EFFECT_COLOR_DATA_ID),
        "live effect color is synched"
    );

    let document = write_entity_chunk(&store, (0, 0)).expect("the save form is well formed");
    assert_eq!(document.entities().next().unwrap().id(), "minecraft:arrow");

    let mut reloaded = EntityStore::new(0, EntityIdCounter::new());
    let ids = read_entity_chunk(&mut reloaded, &document).expect("the save form reads back");
    let after = reloaded.get(ids[0]).unwrap();
    assert_eq!(after.uuid(), before.uuid());
    assert_eq!(after.position(), before.position());
    assert_eq!(after.delta_movement(), before.delta_movement());
    assert!(after.needs_sync());

    let after_data = arrow_data(&reloaded, ids[0]);
    assert_eq!(after_data.pickup(), before_data.pickup());
    assert_eq!(after_data.damage(), before_data.damage());
    assert_eq!(after_data.crit(), before_data.crit());
    assert_eq!(after_data.pierce_level(), before_data.pierce_level());
    assert_eq!(
        after_data.sound_event_resource_id(),
        before_data.sound_event_resource_id()
    );
    assert_eq!(after_data.item(), before_data.item());
    assert_eq!(after_data.life(), before_data.life());
    assert_eq!(after_data.in_ground(), before_data.in_ground());
    assert_eq!(after_data.shake_time(), before_data.shake_time());
    assert_eq!(
        after_data.effect_color(),
        crate::entity::arrow::DEFAULT_EFFECT_COLOR
    );
    assert!(
        non_default_synched_values(after)
            .iter()
            .all(|row| row.data_id != VANILLA_ARROW_EFFECT_COLOR_DATA_ID),
        "effect color is not restored from save"
    );
    let NbtSummaryValue::Compound(entries) = reloaded.command_data(ids[0]).unwrap() else {
        panic!("the restored arrow command data is a compound");
    };
    assert!(entries.iter().any(|entry| entry.name == "SoundEvent"
        && entry.value == NbtSummaryValue::String("minecraft:block.bell.use".into())));

    reloaded.drain_lifecycle_events();
    land_arrow(&mut reloaded, &mut level, ids[0]);
    let expected = bbb_protocol::packets::SoundEventHolder::Reference {
        registry_id: bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
            "minecraft:block.bell.use",
        )
        .unwrap(),
    };
    assert!(reloaded
        .drain_lifecycle_events()
        .iter()
        .any(|event| matches!(event,
            EntityLifecycleEvent::Sound { sound } if sound.sound == expected
        )));
}

fn arrow_data_mut(store: &mut EntityStore, id: i32) -> &mut ArrowData {
    match &mut store.get_mut(id).expect("entity is live").data {
        EntityData::Arrow(data) => data,
        other => panic!("expected an arrow, found {other:?}"),
    }
}

fn four_block_border() -> bbb_server_types::interaction::WorldBorder {
    bbb_server_types::interaction::WorldBorder::from_settings(
        0.0, 0.0, 0.2, 5.0, 5, 15, 4.0, 0, 4.0, 0,
    )
}

#[test]
fn an_arrow_bounces_back_off_the_world_border() {
    let (mut store, mut level, id) = flying_arrow();
    level.set_border(four_block_border());
    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    });
    let mut bounced = false;
    for _ in 0..4 {
        level.with_level(|level| tick_one_entity(&mut store, level, id));
        let entity = store.get(id).expect("the arrow is not consumed");
        assert!(entity.position().x < 2.0);
        bounced |= entity.delta_movement().x < 0.0;
    }
    assert!(bounced);
    assert!(!arrow_data(&store, id).in_ground());
}

#[test]
fn projectiles_pass_through_their_owners_vehicle_before_leaving_the_owner() {
    use crate::entity::{
        HurtingProjectileState, LargeFireballData, SmallFireballData, WitherSkullData,
    };

    let flight = |riding: bool, kind: &str| {
        let mut store = EntityStore::new(11, EntityIdCounter::new());
        let mut level = NativeLevel::new();
        let cow = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 2.5,
                    y: 68.0,
                    z: 0.5,
                },
            )
            .unwrap();
        let rider = store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                Vec3d {
                    x: 2.5,
                    y: 69.4,
                    z: 0.5,
                },
            )
            .unwrap();
        if riding {
            assert!(store.link_existing_passenger_forced(cow, rider));
        }
        let owner = Some(store.get(rider).unwrap().uuid());
        let hurting = HurtingProjectileState::new(owner, 0.0);
        let data = match kind {
            "minecraft:arrow" => {
                let mut data = ArrowData::new(arrow_stack());
                data.projectile.set_owner(owner);
                EntityData::Arrow(data)
            }
            "minecraft:fireball" => EntityData::LargeFireball(LargeFireballData::new(
                hurting,
                plain_stack("minecraft:fire_charge", 1).unwrap(),
                1,
            )),
            "minecraft:small_fireball" => EntityData::SmallFireball(SmallFireballData::new(
                hurting,
                plain_stack("minecraft:fire_charge", 1).unwrap(),
            )),
            "minecraft:wither_skull" => {
                EntityData::WitherSkull(WitherSkullData::new(hurting, false))
            }
            _ => unreachable!(),
        };
        let uuid = store.next_uuid();
        let projectile = store
            .restore(
                uuid,
                bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(kind).unwrap(),
                Vec3d {
                    x: 0.5,
                    y: 68.5,
                    z: 0.5,
                },
                data,
            )
            .unwrap();
        store
            .get_mut(projectile)
            .unwrap()
            .set_delta_movement(Vec3d {
                x: 3.0,
                y: 0.0,
                z: 0.0,
            });
        level.with_level(|level| tick_entities(&mut store, level, true));
        store.get(projectile).map(|entity| entity.position().x)
    };
    for kind in [
        "minecraft:arrow",
        "minecraft:fireball",
        "minecraft:small_fireball",
        "minecraft:wither_skull",
    ] {
        assert!(
            flight(false, kind).is_none_or(|x| x < 2.5),
            "a free cow stops {kind}"
        );
        assert!(
            flight(true, kind).is_some_and(|x| x > 3.0),
            "the owner's mount lets {kind} through"
        );
    }
}

#[test]
fn an_arrow_grazing_an_entity_behind_a_block_edge_misses_it() {
    let flight = |slab: bool| {
        let (mut store, mut level, arrow) = flying_arrow();
        store
            .spawn_general_living(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
                Vec3d {
                    x: 2.5,
                    y: 68.0,
                    z: 0.5,
                },
            )
            .unwrap();
        if slab {
            level.set(
                BlockPos { x: 2, y: 69, z: 0 },
                super::state_with(
                    "minecraft:oak_slab",
                    &[("type", "bottom"), ("waterlogged", "false")],
                ),
            );
        }
        let entity = store.get_mut(arrow).unwrap();
        // Old enough for the full 0.3 entity margin.
        entity.tick_count = 10;
        entity.set_pos(Vec3d {
            x: 0.5,
            y: 69.62,
            z: 0.5,
        });
        entity.set_delta_movement(Vec3d {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        });
        store.refile(arrow);
        level.with_level(|level| tick_entities(&mut store, level, true));
        store.get(arrow).unwrap().position().x
    };
    assert!(flight(false) < 2.5, "a clear graze hits the cow");
    assert!(flight(true) > 3.0, "the slab hides the cow from the graze");
}

#[test]
fn an_arrow_in_a_portal_keeps_only_a_two_tick_cooldown() {
    let mut store = EntityStore::new(14, EntityIdCounter::new());
    let position = Vec3d {
        x: 0.5,
        y: 68.0,
        z: 0.5,
    };
    let id = spawn_arrow(&mut store, position, Vec3d::default(), arrow_stack()).unwrap();
    arrow_data_mut(&mut store, id).set_in_ground(true);
    store.get_mut(id).unwrap().portal_cooldown = 50;
    let mut level = NativeLevel::new();
    level.set(
        BlockPos { x: 0, y: 68, z: 0 },
        state_id("minecraft:nether_portal"),
    );

    level.with_level(|level| tick_one_entity(&mut store, level, id));

    assert_eq!(store.get(id).unwrap().portal_cooldown(), 2);
}
