//! Thrown projectiles landing in a live world: snowball, egg, experience bottle,
//! llama spit and firework rocket hit effects.

use super::*;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_EGG_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
    VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
};
use bbb_server_types::item_stack::RuntimeItemStack;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn joined_world() -> (WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "thrower", 0x7a120e);
    (receiver, world)
}

fn spawn_living(world: &mut ServerWorld, type_id: i32, position: Vec3d) -> i32 {
    world
        .spawn_stored_entity_for_test(
            OVERWORLD,
            type_id,
            position,
            EntityData::GeneralLiving(crate::entity::GeneralLivingData::new(type_id).unwrap()),
        )
        .0
}

fn launch(
    world: &mut ServerWorld,
    type_id: i32,
    position: Vec3d,
    data: EntityData,
    motion: Vec3d,
) -> i32 {
    let (id, _) = world.spawn_stored_entity_for_test(OVERWORLD, type_id, position, data);
    world
        .stored_entity_mut_for_test(OVERWORLD, id)
        .unwrap()
        .set_delta_movement(motion);
    id
}

fn health(world: &ServerWorld, id: i32) -> f32 {
    stored_living(world, OVERWORLD, id).health()
}

/// Ticks until every projectile is gone, returning the entity events sent.
fn tick_until_gone(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    projectiles: &[i32],
) -> Vec<(i32, i8)> {
    let mut entity_events = Vec::new();
    for _ in 0..100 {
        let tick = world.tick(receiver.drain());
        entity_events.extend(tick.events.iter().filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(event),
                ..
            } => Some((event.entity_id, event.event_id)),
            _ => None,
        }));
        if projectiles
            .iter()
            .all(|id| world.stored_entity_for_test(OVERWORLD, *id).is_none())
        {
            return entity_events;
        }
    }
    panic!("a projectile never landed");
}

fn snowball(world: &mut ServerWorld, position: Vec3d, motion: Vec3d) -> i32 {
    launch(
        world,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        position,
        EntityData::Snowball(crate::entity::SnowballData::new(
            plain_stack("minecraft:snowball", 1).unwrap(),
        )),
        motion,
    )
}

#[test]
fn a_snowball_hurts_a_blaze_for_three_and_anything_else_for_nothing() {
    let (receiver, mut world) = joined_world();
    let blaze = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_BLAZE_ID,
        Vec3d {
            x: 10.5,
            y: 64.0,
            z: 6.5,
        },
    );
    let cow = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: -9.5,
            y: 64.0,
            z: 6.5,
        },
    );
    world.tick(receiver.drain());
    let (blaze_health, cow_health) = (health(&world, blaze), health(&world, cow));
    let toward_blaze = snowball(
        &mut world,
        Vec3d {
            x: 10.5,
            y: 64.8,
            z: 4.0,
        },
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    );
    let toward_cow = snowball(
        &mut world,
        Vec3d {
            x: -9.5,
            y: 64.6,
            z: 4.0,
        },
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    );

    let events = tick_until_gone(&receiver, &mut world, &[toward_blaze, toward_cow]);

    assert_eq!(health(&world, blaze), blaze_health - 3.0);
    assert_eq!(health(&world, cow), cow_health);
    assert!(
        stored_living(&world, OVERWORLD, cow).hurt_time > 0,
        "a zero-damage hit still lands"
    );
    assert!(events.contains(&(toward_blaze, 3)), "{events:?}");
}

#[test]
fn eggs_hatch_baby_chickens_of_the_egg_variant() {
    let (receiver, mut world) = joined_world();
    let eggs: Vec<i32> = (0..64)
        .map(|index| {
            launch(
                &mut world,
                VANILLA_ENTITY_TYPE_EGG_ID,
                Vec3d {
                    x: 8.5 + f64::from(index % 8),
                    y: 66.0,
                    z: 8.5 + f64::from(index / 8),
                },
                EntityData::Egg(crate::entity::EggData::new(
                    plain_stack("minecraft:blue_egg", 1).unwrap(),
                )),
                Vec3d {
                    x: 0.0,
                    y: -0.5,
                    z: 0.0,
                },
            )
        })
        .collect();

    tick_until_gone(&receiver, &mut world, &eggs);

    let chickens: Vec<i32> = world
        .stored_entity_ids_for_test(OVERWORLD)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(OVERWORLD, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_CHICKEN_ID)
        })
        .collect();
    assert!(!chickens.is_empty(), "64 eggs at 1/8 hatch at least once");
    for chicken in chickens {
        let NbtSummaryValue::Compound(entries) = world
            .stored_entity_command_data_for_test(OVERWORLD, chicken)
            .unwrap()
        else {
            panic!("an entity record is a compound");
        };
        let field = |name: &str| {
            entries
                .iter()
                .find(|entry| entry.name == name)
                .map(|entry| entry.value.clone())
        };
        assert!(
            matches!(field("Age"), Some(NbtSummaryValue::Int(age)) if age < 0),
            "{:?}",
            field("Age")
        );
        assert_eq!(
            field("variant"),
            Some(NbtSummaryValue::string("minecraft:cold"))
        );
    }
}

#[test]
fn an_experience_bottle_breaks_into_three_to_eleven_experience() {
    let (receiver, mut world) = joined_world();
    let bottle = launch(
        &mut world,
        VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
        Vec3d {
            x: -12.5,
            y: 66.0,
            z: -12.5,
        },
        EntityData::ExperienceBottle(crate::entity::ExperienceBottleData::new(
            plain_stack("minecraft:experience_bottle", 1).unwrap(),
        )),
        Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        },
    );

    tick_until_gone(&receiver, &mut world, &[bottle]);

    let total: i32 = world
        .stored_experience_orbs_for_test(OVERWORLD)
        .iter()
        .map(|(_, _, value, count)| value * count)
        .sum();
    assert!((3..=11).contains(&total), "{total}");
}

#[test]
fn llama_spit_hurts_for_one_on_behalf_of_its_llama() {
    let (receiver, mut world) = joined_world();
    let llama = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        Vec3d {
            x: -10.5,
            y: 64.0,
            z: 10.5,
        },
    );
    let cow = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: -10.5,
            y: 64.0,
            z: 15.5,
        },
    );
    world.tick(receiver.drain());
    let before = health(&world, cow);
    let llama_uuid = stored_uuid(&world, OVERWORLD, llama);
    let mut spit = crate::entity::LlamaSpitData::default();
    spit.set_owner(Some(llama_uuid));
    let spit = launch(
        &mut world,
        VANILLA_ENTITY_TYPE_LLAMA_SPIT_ID,
        Vec3d {
            x: -10.5,
            y: 64.8,
            z: 12.5,
        },
        EntityData::LlamaSpit(spit),
        Vec3d {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    );

    tick_until_gone(&receiver, &mut world, &[spit]);

    assert_eq!(health(&world, cow), before - 1.0);
}

fn exploding_rocket() -> RuntimeItemStack {
    RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:firework_rocket"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:fireworks".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "explosions".into(),
                        value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            NbtSummaryEntry {
                                name: "shape".into(),
                                value: NbtSummaryValue::string("small_ball"),
                            },
                        ])]),
                    }]),
                }]),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

#[test]
fn an_exploding_firework_hurts_nearby_living_entities() {
    let (receiver, mut world) = joined_world();
    let near = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 14.5,
            y: 64.0,
            z: -12.5,
        },
    );
    let far = spawn_living(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 20.5,
            y: 64.0,
            z: -12.5,
        },
    );
    world.tick(receiver.drain());
    let (near_health, far_health) = (health(&world, near), health(&world, far));
    let rocket = launch(
        &mut world,
        VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        Vec3d {
            x: 12.5,
            y: 66.0,
            z: -12.5,
        },
        EntityData::FireworkRocket(crate::entity::FireworkRocketData::new(exploding_rocket())),
        Vec3d {
            x: 0.0,
            y: -0.8,
            z: 0.0,
        },
    );

    let events = tick_until_gone(&receiver, &mut world, &[rocket]);

    assert!(events.contains(&(rocket, 17)), "{events:?}");
    let lost = near_health - health(&world, near);
    assert!(lost > 0.0 && lost <= 7.0, "{lost}");
    assert_eq!(health(&world, far), far_health, "beyond the 5 block radius");
}
