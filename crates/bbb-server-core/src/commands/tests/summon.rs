//! `/summon` construction, NBT and passenger publication.

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id;
use bbb_protocol::NbtSummaryValue;

use super::*;

fn summon_world(difficulty: Difficulty) -> CommandTestWorld {
    CommandTestWorld::default().with_difficulty(difficulty)
}

fn assert_summoned(world: &CommandTestWorld, resource_id: &str, position: [f64; 3]) {
    {
        let reads = world.world().entity_reads();
        let entities: Vec<_> = reads.stored_entities().collect();
        assert_eq!(entities.len(), 1);
        let entity = entities[0].entity;
        assert_eq!(
            entity.entity_type_id(),
            vanilla_entity_type_id_for_resource_id(resource_id).unwrap()
        );
        let actual = entity.position();
        assert_eq!([actual.x, actual.y, actual.z], position);
    }
}

/// `LEVEL_GAMEMASTERS` (`SummonCommand.java:38`); below it the root is gone.
#[test]
fn summon_needs_gamemaster() {
    let refusal = refused("summon minecraft:pig", CommandPermission::Moderator);
    assert_eq!(
        translate_key(failure_messages(&refusal)[0]).as_deref(),
        Some("command.unknown.command")
    );
}

#[test]
fn summon_spawns_a_living_type_at_the_source() {
    let mut world = summon_world(Difficulty::Easy);
    let run = ran("summon minecraft:pig", &mut world);
    assert_eq!(run.result, Some(1));
    assert_eq!(keys(&run), ["commands.summon.success"]);
    assert!(run.feedback[0].inform_admins);
    assert_summoned(&world, "minecraft:pig", [0.0, 0.0, 0.0]);
}

#[test]
fn summon_spawns_at_the_explicit_position() {
    let mut world = summon_world(Difficulty::Easy);
    let run = ran("summon minecraft:pig 3.5 64 1.25", &mut world);
    assert_eq!(run.result, Some(1));
    assert_summoned(&world, "minecraft:pig", [3.5, 64.0, 1.25]);
}

#[test]
fn summon_player_is_not_summonable() {
    let run = ran("summon minecraft:player", summon_world(Difficulty::Easy));
    assert_eq!(keys(&run), ["entity.not_summonable"]);
    assert_eq!(run.result, None);
}

#[test]
fn summon_refuses_missing_or_removed_entities() {
    for command in ["summon minecraft:item", "summon minecraft:item 0 64 0 {}"] {
        let mut world = summon_world(Difficulty::Easy);
        let run = ran(command, &mut world);
        assert_eq!(run.result, None, "{command}: {run:?}");
        assert_eq!(world.world().entity_reads().stored_entities().count(), 0);
    }
}

#[test]
fn summon_leash_knot_and_data_merge_use_attached_geometry() {
    fn assert_geometry(world: &CommandTestWorld, uuid: Uuid, attached: BlockPos, anchor: Vec3d) {
        let reads = world.world().entity_reads();
        let entities: Vec<_> = reads.stored_entities().collect();
        assert_eq!(entities.len(), 1);
        let entity = entities[0].entity;
        assert_eq!(entity.uuid(), uuid);
        assert_eq!(
            entity.entity_type_id(),
            vanilla_entity_type_id_for_resource_id("minecraft:leash_knot").unwrap()
        );
        let crate::entity::EntityData::LeashKnot(knot) = entity.data() else {
            panic!("summoned entity is not a leash knot");
        };
        assert_eq!(knot.attached(), attached);
        assert_eq!(entity.position(), anchor);
        let [width, height] =
            bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity.entity_type_id())
                .unwrap();
        assert_eq!(
            entity.bounding_box(),
            bbb_server_types::geometry::Aabb::of_entity(anchor, width, height)
        );
    }

    for nbt in ["", " {UUID:[I;0,0,0,42],Tags:[tether],Glowing:1b}"] {
        let mut world = summon_world(Difficulty::Easy);
        let run = ran(
            &format!("summon minecraft:leash_knot 3.75 64.8 -1.25{nbt}"),
            &mut world,
        );
        assert_eq!(run.result, Some(1), "{nbt}: {run:?}");
        assert_eq!(keys(&run), ["commands.summon.success"]);
        let uuid = world
            .world()
            .entity_reads()
            .stored_entities()
            .next()
            .unwrap()
            .entity
            .uuid();
        assert_geometry(
            &world,
            uuid,
            BlockPos { x: 3, y: 64, z: -2 },
            Vec3d {
                x: 3.5,
                y: 64.375,
                z: -1.5,
            },
        );
        if nbt.is_empty() {
            continue;
        }
        assert_eq!(uuid, Uuid::from_u128(42));
        for command in [
            "data get entity 00000000-0000-0000-0000-00000000002a Glowing",
            "execute if entity @e[type=leash_knot,tag=tether]",
            "data merge entity 00000000-0000-0000-0000-00000000002a {Silent:1b,Pos:[-2.25d,70.9d,4.75d]}",
            "data get entity 00000000-0000-0000-0000-00000000002a Silent",
            "data get entity 00000000-0000-0000-0000-00000000002a Glowing",
            "execute if entity @e[type=leash_knot,tag=tether]",
        ] {
            assert_eq!(ran(command, &mut world).result, Some(1), "{command}");
        }
        assert_geometry(
            &world,
            uuid,
            BlockPos { x: -3, y: 70, z: 4 },
            Vec3d {
                x: -2.5,
                y: 70.375,
                z: 4.5,
            },
        );
    }
}

#[test]
fn summon_constructs_existing_vehicle_runtimes() {
    for resource_id in [
        "minecraft:acacia_boat",
        "minecraft:bamboo_raft",
        "minecraft:oak_chest_boat",
        "minecraft:minecart",
    ] {
        let mut world = summon_world(Difficulty::Easy);
        let run = ran(&format!("summon {resource_id} 3.5 64 1.25"), &mut world);
        assert_eq!(run.result, Some(1), "{resource_id}: {run:?}");
        assert_summoned(&world, resource_id, [3.5, 64.0, 1.25]);
    }
}

#[test]
fn summon_hostile_in_peaceful_is_failed() {
    let mut world = summon_world(Difficulty::Peaceful);
    let run = ran("summon minecraft:zombie", &mut world);
    assert_eq!(keys(&run), ["commands.summon.failed.peaceful"]);
    assert!(world
        .world()
        .entity_reads()
        .stored_entities()
        .next()
        .is_none());
}

#[test]
fn summon_hostile_outside_peaceful_succeeds() {
    let mut world = summon_world(Difficulty::Easy);
    let run = ran("summon minecraft:zombie", &mut world);
    assert_eq!(keys(&run), ["commands.summon.success"]);
    assert_eq!(run.result, Some(1));
    assert_summoned(&world, "minecraft:zombie", [0.0, 0.0, 0.0]);
}

#[test]
fn summon_refuses_a_position_outside_spawnable_bounds() {
    let run = ran(
        "summon minecraft:pig 0 20000000 0",
        summon_world(Difficulty::Easy),
    );
    assert_eq!(keys(&run), ["commands.summon.invalidPosition"]);
}

#[test]
fn summon_unknown_type_is_resource_not_found() {
    let run = ran("summon minecraft:missing", summon_world(Difficulty::Easy));
    assert_eq!(keys(&run), ["argument.resource.not_found", CURSOR_CONTEXT]);
}

#[test]
fn summon_nbt_restores_fields_and_passengers_at_the_command_position() {
    let mut world = summon_world(Difficulty::Easy);
    let run = ran(
        r#"summon minecraft:pig 3.5 64 1.25 {id:"minecraft:zombie",UUID:[I;0,0,0,42],Pos:[99d,99d,99d],Rotation:[45f,15f],Age:-24000,Health:7f,NoAI:1b,Passengers:[{id:cow,UUID:[I;0,0,0,43],Age:-1200,Passengers:[{id:chicken,UUID:[I;0,0,0,44],NoAI:1b}]}]}"#,
        &mut world,
    );
    assert_eq!(run.result, Some(1), "{run:?}");
    for command in [
        "execute as 00000000-0000-0000-0000-00000000002a if entity @s[type=pig]",
        "execute as 00000000-0000-0000-0000-00000000002a on passengers if entity @s[type=cow]",
        "execute as 00000000-0000-0000-0000-00000000002a on passengers on passengers if entity @s[type=chicken]",
    ] {
        assert_eq!(ran(command, &mut world).result, Some(1), "{command}");
    }
    for (path, expected) in [
        ("Age", -24000),
        ("Health", 7),
        ("NoAI", 1),
        ("Rotation[0]", 45),
    ] {
        assert_eq!(
            ran(
                &format!("data get entity 00000000-0000-0000-0000-00000000002a {path}"),
                &mut world
            )
            .result,
            Some(expected),
            "{path}"
        );
    }
    let saved = world
        .world()
        .entity_reads()
        .entity_data(Uuid::from_u128(42))
        .unwrap()
        .unwrap();
    let mut restored = summon_world(Difficulty::Easy);
    let run = ran(
        &format!(
            "summon minecraft:pig 3.5 64 1.25 {}",
            bbb_protocol::canonical_snbt_string(&saved)
        ),
        &mut restored,
    );
    assert_eq!(run.result, Some(1), "{run:?}");
    assert_eq!(
        restored
            .world()
            .entity_reads()
            .entity_data(Uuid::from_u128(42))
            .unwrap(),
        Some(saved)
    );
    for fixture in [&world, &restored] {
        let reads = fixture.world().entity_reads();
        let entities: Vec<_> = reads.stored_entities().collect();
        assert_eq!(entities.len(), 3);
        for stored in entities {
            let actual = stored.entity.position();
            assert_eq!([actual.x, actual.y, actual.z], [3.5, 64.0, 1.25]);
        }
    }
    assert_eq!(
        ran(
            "execute as 00000000-0000-0000-0000-00000000002a on passengers on passengers if entity @s[type=chicken]",
            &mut restored
        )
        .result,
        Some(1)
    );
}

#[test]
fn summon_negative_age_frogs_and_parrots_keep_adult_geometry_and_passengers() {
    for resource_id in ["minecraft:frog", "minecraft:parrot"] {
        let mut world = summon_world(Difficulty::Easy);
        let run = ran(
            &format!(
                "summon {resource_id} 3.5 64 1.25 {{UUID:[I;0,0,0,42],Age:-24000,NoAI:1b,Passengers:[{{id:chicken,NoAI:1b}}]}}"
            ),
            &mut world,
        );
        assert_eq!(run.result, Some(1), "{resource_id}: {run:?}");
        let reads = world.world().entity_reads();
        let entity = reads
            .stored_entities()
            .find(|stored| stored.entity.uuid() == Uuid::from_u128(42))
            .unwrap()
            .entity;
        let [width, height] =
            bbb_protocol::entity_types::vanilla_entity_type_dimensions(entity.entity_type_id())
                .unwrap();
        assert_eq!([entity.bb_width(), entity.bb_height()], [width, height]);
        assert_eq!(
            ran(
                "data get entity 00000000-0000-0000-0000-00000000002a Age",
                &mut world
            )
            .result,
            Some(-24000)
        );
        assert_eq!(
            ran(
                "execute as 00000000-0000-0000-0000-00000000002a on passengers if entity @s[type=chicken]",
                &mut world
            )
            .result,
            Some(1)
        );
    }
}

#[test]
fn summon_state_scaled_fish_keep_passengers_and_round_trip_saved_fields() {
    for (resource_id, field, value) in [
        (
            "minecraft:salmon",
            "type",
            NbtSummaryValue::String("small".into()),
        ),
        (
            "minecraft:salmon",
            "type",
            NbtSummaryValue::String("large".into()),
        ),
        ("minecraft:pufferfish", "PuffState", NbtSummaryValue::Int(0)),
        ("minecraft:pufferfish", "PuffState", NbtSummaryValue::Int(2)),
    ] {
        let mut world = summon_world(Difficulty::Easy);
        let run = ran(
            &format!(
                "summon {resource_id} 3.5 64 1.25 {{UUID:[I;0,0,0,42],NoAI:1b,NoGravity:1b,{field}:{},Passengers:[{{id:chicken,UUID:[I;0,0,0,43],NoAI:1b,NoGravity:1b}}]}}",
                bbb_protocol::canonical_snbt_string(&value)
            ),
            &mut world,
        );
        assert_eq!(run.result, Some(1), "{resource_id} {value:?}: {run:?}");
        let passenger_query = "execute as 00000000-0000-0000-0000-00000000002a on passengers if entity @s[type=chicken]";
        assert_eq!(ran(passenger_query, &mut world).result, Some(1));
        let saved = world
            .world()
            .entity_reads()
            .entity_data(Uuid::from_u128(42))
            .unwrap()
            .unwrap();
        assert_eq!(crate::loot_runtime::nbt_field(&saved, field), Some(&value));
        let mut restored = summon_world(Difficulty::Easy);
        let run = ran(
            &format!(
                "summon {resource_id} 3.5 64 1.25 {}",
                bbb_protocol::canonical_snbt_string(&saved)
            ),
            &mut restored,
        );
        assert_eq!(run.result, Some(1), "{resource_id} {value:?}: {run:?}");
        assert_eq!(ran(passenger_query, &mut restored).result, Some(1));
        assert_eq!(
            restored
                .world()
                .entity_reads()
                .entity_data(Uuid::from_u128(42))
                .unwrap(),
            Some(saved)
        );
        for fixture in [&world, &restored] {
            let reads = fixture.world().entity_reads();
            let entities = reads.stored_entities().collect::<Vec<_>>();
            assert_eq!(entities.len(), 2);
            assert!(entities
                .iter()
                .any(|stored| stored.entity.uuid() == Uuid::from_u128(42)
                    && stored.entity.entity_type_id()
                        == vanilla_entity_type_id_for_resource_id(resource_id).unwrap()));
            assert!(entities
                .iter()
                .any(|stored| stored.entity.uuid() == Uuid::from_u128(43)
                    && stored.entity.entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHICKEN_ID));
        }
    }
}

#[test]
fn summon_nbt_refuses_existing_entity_and_player_uuids_in_the_tree() {
    let mut world = world(&[user("alice", 1)]).with_difficulty(Difficulty::Easy);
    world.setup("summon minecraft:pig 3.5 64 1.25 {UUID:[I;0,0,0,42],Age:-1200}");
    let before = world
        .world()
        .entity_reads()
        .entity_data(Uuid::from_u128(42))
        .unwrap();
    for nbt in [
        "{UUID:[I;0,0,0,42]}",
        "{UUID:[I;0,0,0,1]}",
        "{Passengers:[{id:cow,UUID:[I;0,0,0,42]}]}",
        "{Passengers:[{id:cow,UUID:[I;0,0,0,1]}]}",
    ] {
        let run = ran(&format!("summon minecraft:pig 8 64 8 {nbt}"), &mut world);
        assert_eq!(run.result, None, "{nbt}: {run:?}");
        assert_eq!(keys(&run), ["commands.summon.failed.uuid"], "{nbt}");
        let reads = world.world().entity_reads();
        assert_eq!(reads.stored_entities().count(), 1);
        assert_eq!(reads.entity_data(Uuid::from_u128(42)).unwrap(), before);
    }
    assert_eq!(
        ran("data get entity alice Health", &mut world).result,
        Some(20)
    );
}

#[test]
fn summon_explicit_empty_nbt_skips_mob_finalization() {
    for nbt in ["", " {}"] {
        let mut world = summon_world(Difficulty::Easy);
        let run = ran(
            &format!("summon minecraft:piglin_brute 0 64 0{nbt}"),
            &mut world,
        );
        assert_eq!(run.result, Some(1), "{run:?}");
        let uuid = world
            .world()
            .entity_reads()
            .stored_entities()
            .next()
            .unwrap()
            .entity
            .uuid();
        let run = ran(
            &format!("data get entity {uuid} equipment.mainhand.id"),
            &mut world,
        );
        assert_eq!(run.result.is_some(), nbt.is_empty(), "{nbt}: {run:?}");
    }
}

#[test]
fn summon_lightning_nbt_reads_base_fields_without_a_saved_chunk() {
    let mut world = summon_world(Difficulty::Easy);
    let run = ran(
        "summon minecraft:lightning_bolt 3.5 64 1.25 {UUID:[I;0,0,0,42],Silent:1b,Rotation:[45f,15f],Tags:[strike]}",
        &mut world,
    );
    assert_eq!(run.result, Some(1), "{run:?}");
    assert_summoned(&world, "minecraft:lightning_bolt", [3.5, 64.0, 1.25]);
    for (path, expected) in [("Silent", 1), ("Rotation[0]", 45)] {
        assert_eq!(
            ran(
                &format!("data get entity 00000000-0000-0000-0000-00000000002a {path}"),
                &mut world
            )
            .result,
            Some(expected)
        );
    }
    assert_eq!(
        ran(
            "execute if entity @e[type=lightning_bolt,tag=strike]",
            &mut world
        )
        .result,
        Some(1)
    );
}

fn mentions(value: &NbtSummaryValue, needle: &str) -> bool {
    match value {
        NbtSummaryValue::String(text) => text == needle,
        NbtSummaryValue::Compound(entries) => entries
            .iter()
            .any(|entry| entry.name != "hover_event" && mentions(&entry.value, needle)),
        NbtSummaryValue::List(values) => values.iter().any(|value| mentions(value, needle)),
        _ => false,
    }
}

/// `Entity.getDisplayName` (Entity.java:3207-3209): feedback names an entity by
/// its custom name under its team's prefix, and `execute as` speaks with it.
#[test]
fn feedback_and_execute_as_use_the_entity_display_name() {
    let mut world = summon_world(Difficulty::Easy);
    let summoned = ran(
        "summon minecraft:armor_stand 0 64 0 {CustomName:\"Bob\"}",
        &mut world,
    );
    assert_eq!(keys(&summoned), ["commands.summon.success"]);
    assert!(mentions(&arguments(&summoned, 0)[0], "Bob"));
    world.setup("team add red");
    world.setup("team modify red prefix \"[R] \"");
    world.setup("team join red @e[type=minecraft:armor_stand]");
    let tagged = ran("tag @e[type=minecraft:armor_stand] add t", &mut world);
    let name = &arguments(&tagged, 0)[1];
    assert!(mentions(name, "Bob") && mentions(name, "[R] "), "{name:?}");
    let said = ran(
        "execute as @e[type=minecraft:armor_stand] run say hi",
        &mut world,
    );
    let message = chat_broadcast_message(&said.broadcast[0]);
    assert_eq!(
        translate_key(&message).as_deref(),
        Some("chat.type.announcement")
    );
    assert!(mentions(&message, "Bob") && mentions(&message, "[R] "));
}

/// `Player.getDisplayName` (Player.java:1676-1679) formats the name for the
/// player's team.
#[test]
fn player_feedback_carries_the_team_prefix() {
    let mut world = world(&[user("alice", 1)]);
    world.setup("team add red");
    world.setup("team modify red prefix \"[R] \"");
    world.setup("team join red alice");
    let run = ran("experience query alice levels", &mut world);
    let name = &arguments(&run, 0)[0];
    assert!(
        mentions(name, "alice") && mentions(name, "[R] "),
        "{name:?}"
    );
}
