//! Job-site handoff uses the receiving villager's route through the real world.

use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID as VILLAGER;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn record(world: &ServerWorld, id: i32) -> Vec<NbtSummaryEntry> {
    let uuid = world.stored_entity_for_test(OVERWORLD, id).unwrap().uuid();
    let NbtSummaryValue::Compound(entries) = world
        .entity_reads()
        .entity_data(uuid)
        .expect("the villager command data saves")
        .expect("the villager is present")
    else {
        panic!("entity record is a compound")
    };
    entries
}

fn at<'a>(entries: &'a [NbtSummaryEntry], path: &[&str]) -> Option<&'a NbtSummaryValue> {
    let value = &entries.iter().find(|entry| entry.name == path[0])?.value;
    if path.len() == 1 {
        Some(value)
    } else if let NbtSummaryValue::Compound(children) = value {
        at(children, &path[1..])
    } else {
        None
    }
}

#[test]
fn yield_job_site_skips_an_enclosed_librarian_for_a_reachable_one() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "jobs", 0x7ade);
    let origin = world.publication().players[0].position;
    let (x, y, z) = (
        origin.x.floor() as i32 + 4,
        origin.y.floor() as i32,
        origin.z.floor() as i32,
    );
    let mut sequence = 1;
    let mut command = |world: &mut ServerWorld, text: &str| {
        sequence += 1;
        console_command(&sender, &receiver, world, sequence, text);
    };
    command(&mut world, "gamerule advance_time false");
    command(&mut world, "time set noon");
    command(
        &mut world,
        &format!("forceload add {} {} {} {}", x - 4, z - 5, x + 14, z + 8),
    );
    command(
        &mut world,
        &format!(
            "fill {} {} {} {} {} {} minecraft:stone",
            x - 4,
            y - 1,
            z - 5,
            x + 14,
            y - 1,
            z + 8
        ),
    );
    command(
        &mut world,
        &format!(
            "fill {} {y} {} {} {} {} minecraft:air",
            x - 4,
            z - 5,
            x + 14,
            y + 4,
            z + 8
        ),
    );
    let spawn = |world: &mut ServerWorld, dx: i32, dz: i32| {
        world
            .spawn_stored_entity_for_test(
                OVERWORLD,
                VILLAGER,
                Vec3d {
                    x: f64::from(x + dx) + 0.5,
                    y: f64::from(y),
                    z: f64::from(z + dz) + 0.5,
                },
                crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(VILLAGER).unwrap()),
            )
            .0
    };
    let source = spawn(&mut world, -2, 0);
    command(
        &mut world,
        "attribute @e[type=minecraft:villager,limit=1] minecraft:movement_speed base set 0",
    );
    command(
        &mut world,
        &format!("setblock {} {y} {z} minecraft:lectern", x + 10),
    );
    let memory_path = |name| ["Brain", "memories", name, "value"];
    let mut acquired = false;
    for _ in 0..200 {
        world.tick(receiver.drain());
        if at(
            &record(&world, source),
            &memory_path("minecraft:potential_job_site"),
        )
        .is_some()
        {
            acquired = true;
            break;
        }
    }
    assert!(acquired, "the unemployed villager reserves the lectern");

    command(
        &mut world,
        &format!(
            "fill {x} {y} {} {} {} {} minecraft:stone",
            z - 1,
            x + 2,
            y + 3,
            z + 1
        ),
    );
    command(
        &mut world,
        &format!(
            "fill {} {y} {z} {} {} {z} minecraft:air",
            x + 1,
            x + 1,
            y + 1
        ),
    );
    let enclosed = spawn(&mut world, 1, 0);
    let reachable = spawn(&mut world, 1, 4);
    for id in [enclosed, reachable] {
        let mut entries = record(&world, id);
        entries.retain(|entry| entry.name != "VillagerData" && entry.name != "Xp");
        entries.push(NbtSummaryEntry {
            name: "Offers".into(),
            value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "Recipes".into(),
                value: NbtSummaryValue::List(Vec::new()),
            }]),
        });
        entries.push(NbtSummaryEntry {
            name: "Xp".into(),
            value: NbtSummaryValue::Int(1),
        });
        entries.push(NbtSummaryEntry {
            name: "VillagerData".into(),
            value: NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String("minecraft:plains".into()),
                },
                NbtSummaryEntry {
                    name: "profession".into(),
                    value: NbtSummaryValue::String("minecraft:librarian".into()),
                },
                NbtSummaryEntry {
                    name: "level".into(),
                    value: NbtSummaryValue::Int(1),
                },
            ]),
        });
        world.load_stored_entity_record_for_test(OVERWORLD, id, &entries);
    }
    let mut claimed = false;
    for _ in 0..600 {
        world.tick(receiver.drain());
        if at(
            &record(&world, reachable),
            &memory_path("minecraft:job_site"),
        )
        .is_some()
        {
            claimed = true;
            break;
        }
    }
    assert!(
        claimed,
        "the reachable librarian must receive the reserved workstation"
    );
    assert!(at(
        &record(&world, enclosed),
        &memory_path("minecraft:job_site")
    )
    .is_none());
    let saved = record(&world, reachable);
    assert_eq!(
        at(
            &saved,
            &["Brain", "memories", "minecraft:job_site", "value", "pos"]
        ),
        Some(&NbtSummaryValue::IntArray(vec![x + 10, y, z]))
    );
    assert_eq!(
        at(&saved, &["VillagerData", "profession"]),
        Some(&NbtSummaryValue::String("minecraft:librarian".into()))
    );
}
