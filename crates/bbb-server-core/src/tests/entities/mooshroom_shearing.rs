use super::feeding::interact_entity_packet;
use super::horse_interaction::{hold, item_id, record};
use super::*;
use bbb_protocol::{NbtSummaryEntry as Entry, NbtSummaryValue as Nbt};

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const MOOSHROOM: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MOOSHROOM_ID;
const COW: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;

fn spawn(world: &mut ServerWorld, position: Vec3d, fields: &str) -> i32 {
    let id = world
        .spawn_stored_entity_for_test(
            DIM,
            MOOSHROOM,
            position,
            crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(MOOSHROOM).unwrap()),
        )
        .0;
    let Nbt::Compound(fields) = bbb_protocol::parse_snbt_compound(fields).unwrap() else {
        unreachable!()
    };
    let mut entries = record(world, id);
    for field in fields.into_iter().chain([Entry {
        name: "NoAI".into(),
        value: Nbt::Byte(1),
    }]) {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(DIM, id, &entries);
    id
}

fn cows(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(DIM, *id)
                .unwrap()
                .entity_type_id()
                == COW
        })
        .collect()
}

fn drops(world: &ServerWorld) -> Vec<(i32, i32)> {
    world
        .stored_entity_ids_for_test(DIM)
        .into_iter()
        .filter_map(|id| {
            let crate::entity::EntityData::Item(data) =
                world.stored_entity_for_test(DIM, id)?.data()
            else {
                return None;
            };
            Some((data.item().item_id()?, data.item().count()))
        })
        .collect()
}

#[test]
fn live_mooshroom_shearing_converts_both_variants_and_preserves_common_saved_state() {
    for (variant, game_type) in [("red", GameType::Survival), ("brown", GameType::Creative)] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.game_type = game_type;
        let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
        let opposite_variant = if variant == "red" { "brown" } else { "red" };
        for (id, target, required_variant, reward) in [
            (
                "test:shear_mooshroom",
                "minecraft:mooshroom",
                Some(variant),
                7,
            ),
            (
                "test:shear_opposite_mooshroom",
                "minecraft:mooshroom",
                Some(opposite_variant),
                0,
            ),
            ("test:shear_cow", "minecraft:cow", None, 0),
        ] {
            let mut entity = serde_json::json!({"type":target,"effects":{"minecraft:speed":{}}});
            if let Some(required_variant) = required_variant {
                entity["components"] =
                    serde_json::json!({"minecraft:mooshroom/variant":required_variant});
            }
            startup
                .advancements
                .advancements
                .push(bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(&serde_json::json!({
                    "criteria":{"sheared":{
                        "trigger":"minecraft:player_interacted_with_entity",
                        "conditions":{
                            "entity":entity,
                            "item":{"items":"minecraft:shears","components":{"minecraft:damage":0}}
                        }
                    }},
                    "rewards":{"experience":reward}
                }))
                .unwrap(),
            });
        }
        let mut world = spawn_pinned_world(config);
        join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x50ee);
        if variant == "brown" {
            join_loaded(&sender, &receiver, &mut world, 2, "rider", 0x50ef);
        }
        world.tick_rate_mut().set_frozen(true);
        let position = Vec3d {
            z: world.publication().players[0].position.z + 1.0,
            ..world.publication().players[0].position
        };
        let id = spawn(
            &mut world,
            position,
            &format!(
                r#"{{Type:"{variant}",Health:2.0f,Age:120,ForcedAge:7,Silent:1b,PersistenceRequired:1b,CanPickUpLoot:1b,equipment:{{mainhand:{{id:"minecraft:stone",count:1}}}},CustomName:{{text:"Moo"}},CustomNameVisible:1b,Tags:["keep"],data:{{value:17}},active_effects:[{{id:"minecraft:speed",duration:-1,amplifier:0b}}]}}"#
            ),
        );
        let pig = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
        let mut pigs = Vec::new();
        let mut pig_ids = Vec::new();
        for _ in 0..2 {
            let pig_id = world
                .spawn_stored_entity_for_test(
                    DIM,
                    pig,
                    position,
                    crate::entity::EntityData::GeneralLiving(GeneralLivingData::new(pig).unwrap()),
                )
                .0;
            pig_ids.push(pig_id);
            pigs.push(world.stored_entity_for_test(DIM, pig_id).unwrap().uuid());
        }
        let source_uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        for command in [
            format!("ride {source_uuid} mount {}", pigs[0]),
            if variant == "brown" {
                format!("ride rider mount {source_uuid}")
            } else {
                format!("ride {} mount {source_uuid}", pigs[1])
            },
        ] {
            let tick = console_command(&sender, &receiver, &mut world, 8, &command);
            let answer = console_command_answer(&tick);
            assert_eq!(answer.0, Some(1), "{answer:?}");
        }
        let original = record(&world, id);
        assert!(original.iter().any(|entry| entry.name == "equipment"));
        let old_uuid = world.stored_entity_for_test(DIM, id).unwrap().uuid();
        console_command(&sender, &receiver, &mut world, 10, "team add herd");
        console_command(
            &sender,
            &receiver,
            &mut world,
            11,
            &format!("team join herd {old_uuid}"),
        );
        let position = world.stored_entity_for_test(DIM, id).unwrap().position();
        hold(&mut world, "minecraft:shears", 1);
        let before_events = world.game_events(DIM).1;
        let tick = play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert!(world.stored_entity_for_test(DIM, id).is_none());
        assert!(world.player_advancement_is_done(ConnectionId(1), "test:shear_mooshroom"));
        assert!(!world.player_advancement_is_done(ConnectionId(1), "test:shear_opposite_mooshroom"));
        assert!(!world.player_advancement_is_done(ConnectionId(1), "test:shear_cow"));
        assert_eq!(world.experience_total_for_test(ConnectionId(1)), 7);
        assert!(world
            .live_entity_id_for_uuid_for_test(DIM, old_uuid)
            .is_none());
        let cow_ids = cows(&world);
        assert_eq!(cow_ids.len(), 1);
        let cow = world.stored_entity_for_test(DIM, cow_ids[0]).unwrap();
        assert_eq!(cow.position(), position);
        assert_eq!(cow.living().unwrap().health(), 10.0);
        let new_uuid = cow.uuid();
        console_command(
            &sender,
            &receiver,
            &mut world,
            12,
            "scoreboard objectives add transfer dummy",
        );
        let vehicle_record = record(&world, pig_ids[0]);
        assert!(
            vehicle_record
                .iter()
                .any(|field| field.name == "Passengers"),
            "{vehicle_record:?}"
        );
        for (relation, holder) in [
            ("vehicle", pigs[0].to_string()),
            (
                "passengers",
                if variant == "brown" {
                    "rider".into()
                } else {
                    pigs[1].to_string()
                },
            ),
        ] {
            console_command(
                &sender,
                &receiver,
                &mut world,
                13,
                &format!(
                    "execute as {new_uuid} on {relation} run scoreboard players set @s transfer 1"
                ),
            );
            assert_eq!(
                world
                    .scoreboard()
                    .score(&holder, "transfer")
                    .map(|score| score.value),
                Some(1),
                "{variant}: {relation}"
            );
        }

        let converted = record(&world, cow_ids[0]);
        assert!(!converted.iter().any(|entry| entry.name == "equipment"));
        assert!(converted.iter().any(|entry| entry.name == "Age"
            && matches!(entry.value, Nbt::Int(age) if age > 0 && age <= 120)));

        for field in [
            "CustomName",
            "CustomNameVisible",
            "active_effects",
            "ForcedAge",
            "Silent",
            "NoAI",
            "PersistenceRequired",
            "Tags",
            "data",
        ] {
            assert_eq!(
                converted.iter().find(|entry| entry.name == field),
                original.iter().find(|entry| entry.name == field),
                "{field}"
            );
        }
        assert!(converted
            .iter()
            .find(|entry| entry.name == "CanPickUpLoot")
            .is_none_or(|entry| entry.value == Nbt::Byte(0)));
        let scoreboard = world.scoreboard_mut().board();
        assert!(scoreboard.players_team(&old_uuid.to_string()).is_none());
        assert_eq!(
            scoreboard.players_team(&new_uuid.to_string()).unwrap().name,
            "herd"
        );
        let mushrooms = drops(&world);
        assert_eq!(mushrooms.len(), 5);
        assert!(mushrooms
            .iter()
            .all(|drop| *drop == (item_id(&format!("minecraft:{variant}_mushroom")), 1)));
        assert_eq!(
            tick.snapshot.players[0].inventory.slots[0]
                .component_patch
                .damage,
            if game_type == GameType::Survival {
                Some(1)
            } else {
                None
            }
        );
        assert!(tick.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::Sound(sound))
            if sound.source == SoundSource::Players && sound.volume == 1.0 && sound.pitch == 1.0
                && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.mooshroom.shear"))
        )));
        assert!(tick.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::LevelParticles(particle))
            if particle.count == 1 && particle.particle.particle_type_id == bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_EXPLOSION_ID
                && particle.position.x == position.x && particle.position.z == position.z && particle.position.y > position.y
        )));
        assert!(!tick.events.iter().any(|event| matches!(
            event.sent_packet(),
            Some(ClientboundPlayPacket::EntityAnimation(animation))
            if animation.id == world.player_entity_id_for_test(ConnectionId(1))
        )));
        let (events, count) = world.game_events(DIM);
        assert_eq!(
            count,
            before_events + if variant == "brown" { 4 } else { 2 }
        );
        if variant == "brown" {
            for kind in [
                VanillaGameEvent::ENTITY_DISMOUNT,
                VanillaGameEvent::ENTITY_MOUNT,
            ] {
                assert!(events.iter().any(|event| {
                    event.game_event == kind
                        && event.source_entity_id
                            == Some(world.player_entity_id_for_test(ConnectionId(2)))
                        && event.pos
                            == BlockPos {
                                x: position.x.floor() as i32,
                                y: position.y.floor() as i32,
                                z: position.z.floor() as i32,
                            }
                }));
            }
        }
        for kind in [VanillaGameEvent::SHEAR, VanillaGameEvent::ENTITY_INTERACT] {
            assert!(events.iter().any(|event| event.game_event == kind
                && event.source_entity_id
                    == Some(world.player_entity_id_for_test(ConnectionId(1)))
                && event.pos
                    == BlockPos {
                        x: position.x.floor() as i32,
                        y: position.y.floor() as i32,
                        z: position.z.floor() as i32
                    }));
        }
    }
}

#[test]
fn live_mooshroom_shearing_uses_original_variant_tool_and_location_in_custom_loot() {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    let startup = Arc::make_mut(config.datapacks.as_mut().unwrap());
    let registry = startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap();
    let element = registry
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:shearing/mooshroom")
        .unwrap();
    element.value = bbb_server_datapack::loot::LootElementValue::Table(
        bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
            "type": "minecraft:shearing", "pools": [{ "rolls": 1,
                "conditions": [
                    { "condition": "minecraft:match_tool", "predicate": { "items": "minecraft:shears" } },
                    { "condition": "minecraft:location_check", "predicate": { "block": { "blocks": "minecraft:air" } } },
                    { "condition": "minecraft:entity_properties", "entity": "this", "predicate": { "components": { "minecraft:mooshroom/variant": "brown" } } }
                ],
                "entries": [{ "type": "minecraft:item", "name": "minecraft:diamond", "functions": [{ "function": "minecraft:set_count", "count": 3 }] }]
            }]
        }), None).unwrap()
    );
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "shearer", 0x50ef);
    world.tick_rate_mut().set_frozen(true);
    let position = Vec3d {
        z: world.publication().players[0].position.z + 1.0,
        ..world.publication().players[0].position
    };
    for variant in ["brown", "red"] {
        let id = spawn(&mut world, position, &format!(r#"{{Type:"{variant}"}}"#));
        hold(&mut world, "minecraft:shears", 1);
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            interact_entity_packet(id),
        );
        assert!(world.stored_entity_for_test(DIM, id).is_none());
        assert_eq!(drops(&world), vec![(item_id("minecraft:diamond"), 1); 3]);
    }
}

#[test]
fn dispenser_mooshroom_shearing_converts_adults_and_refuses_babies_and_dead_mobs() {
    for (fields, expected) in [
        (r#"{Type:"brown"}"#, true),
        ("{Age:-100}", false),
        ("{Health:0.0f}", false),
    ] {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(datapack_config());
        join_loaded(&sender, &receiver, &mut world, 1, "viewer", 0x50ef);
        console_command(&sender, &receiver, &mut world, 1, "forceload add 0 0");
        console_command(
            &sender,
            &receiver,
            &mut world,
            2,
            "setblock 0 63 1 minecraft:stone",
        );
        let id = spawn(
            &mut world,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 1.5,
            },
            fields,
        );
        console_command(
            &sender,
            &receiver,
            &mut world,
            3,
            r#"setblock 0 64 0 minecraft:dispenser[facing=south]{Items:[{Slot:0b,id:"minecraft:shears",count:1}]}"#,
        );
        console_command(
            &sender,
            &receiver,
            &mut world,
            4,
            "setblock 1 64 0 minecraft:redstone_block",
        );
        world.tick_rate_mut().set_frozen(false);
        let mut sound = false;
        for _ in 0..5 {
            let tick = world.tick(receiver.drain());
            sound |= tick.events.iter().any(|event| matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
                if sound.source == SoundSource::Blocks && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                    if vanilla_sound_event_resource_id(registry_id) == Some("minecraft:entity.mooshroom.shear"))
            ));
        }
        assert_eq!(cows(&world).len(), usize::from(expected));
        assert_eq!(sound, expected);
        if expected {
            assert!(world.stored_entity_for_test(DIM, id).is_none());
            let mushrooms = drops(&world);
            assert_eq!(mushrooms.iter().map(|(_, count)| count).sum::<i32>(), 5);
            assert!(mushrooms
                .iter()
                .all(|(id, _)| *id == item_id("minecraft:brown_mushroom")));
        } else {
            assert!(world.stored_entity_for_test(DIM, id).is_some());
            assert!(drops(&world).is_empty());
        }
        let shears = world.with_level_for_test(DIM, |level| {
            let container = level
                .block_container(BlockPos { x: 0, y: 64, z: 0 }, true)
                .unwrap();
            level.get_container_item(container, 0).unwrap().unwrap()
        });
        assert_eq!(
            shears.summary().component_patch.damage,
            if expected { Some(1) } else { None }
        );
    }
}
