//! `LivingEntity.dropAllDeathLoot` and
//! `createWitherRose` reached by a real stored mob death: the
//! loot table's own stacks as item entities, the equipment drop chances, the
//! wither rose, and what a refused table leaves behind.

use super::*;

mod effects;
mod entity_conditions;
mod entity_type_tags;
mod fox;
mod item_tags;
mod location;
mod targeted_entity;
mod vehicle_slots;
mod wither_boss;

use crate::entity::{EntityData, GeneralLivingData};
use crate::tests::entities::{console_command, console_command_answer, restore_entity_writes};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_COW_ID,
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_PARCHED_ID,
    VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_SKELETON_ID,
    VANILLA_ENTITY_TYPE_SPIDER_ID, VANILLA_ENTITY_TYPE_STRAY_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
    VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;

/// One stored item entity as the tests here read it: `(entity id, position,
/// item resource id, count, pickup delay, damage component)`.
type StoredItem = (i32, Vec3d, String, i32, i32, Option<i32>);

/// The mob's own position, which `Entity.spawnAtLocation` drops at with its
/// zero offset (`Entity.java:2131-2140`).
const MOB_POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
/// The cell that position recentres to, where `createWitherRose` writes
/// (`LivingEntity.java:1465-1469`).
const MOB_BLOCK: BlockPos = BlockPos { x: 0, y: 64, z: 0 };

fn spawn_mob(world: &mut ServerWorld, entity_type_id: i32) -> i32 {
    helpers::spawn_living_at(world, entity_type_id, MOB_POSITION).0
}

fn items(world: &ServerWorld) -> Vec<StoredItem> {
    world.stored_items_for_test(Java26BuiltinDimension::Overworld)
}

/// The item resource ids the mob's death left on the ground, sorted so two
/// pools of one table compare independently of draw order.
fn dropped_ids(world: &ServerWorld) -> Vec<String> {
    let mut ids: Vec<String> = items(world).into_iter().map(|item| item.2).collect();
    ids.sort();
    ids
}

/// Stages the mob one blow from death, so the command's damage is lethal.
fn stage_one_blow_from_death(world: &mut ServerWorld, mob: i32) {
    helpers::stage_one_blow_from_death(world, mob);
}

/// The lethal hit through the command-damage entry `/kill` uses
/// (`KillCommand` over `LivingEntity.kill`,
/// `LivingEntity`), without the tick a chat command runs.
fn kill_stored(world: &mut ServerWorld, mob: i32) {
    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            mob,
            "minecraft:generic",
            40.0,
            None,
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);
}

/// Loads one record into the stored mob and puts it back where the fixture
/// stood: a record with no `Pos` recentres the entity on the origin.
fn load_record(world: &mut ServerWorld, mob: i32, entries: Vec<NbtSummaryEntry>) {
    world.load_stored_entity_record_for_test(Java26BuiltinDimension::Overworld, mob, &entries);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, mob)
        .expect("the mob is stored")
        .set_pos(MOB_POSITION);
}

fn nbt_entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

/// `drop_chances` as `DropChances.CODEC` writes it,
/// which `Mob.readAdditionalSaveData` reads.
fn drop_chances(slot: &str, chance: f32) -> Vec<NbtSummaryEntry> {
    vec![nbt_entry(
        "drop_chances",
        NbtSummaryValue::Compound(vec![nbt_entry(
            slot,
            NbtSummaryValue::Float(chance.to_bits()),
        )]),
    )]
}

fn equip(world: &mut ServerWorld, mob: i32, item: &str) {
    let stack = crate::entity::item::plain_stack(item, 1).expect("a plain stack decodes");
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, mob)
        .expect("the mob is stored")
        .living_mut()
        .expect("the mob is living")
        .set_item_slot(EquipmentSlot::MainHand, Some(stack));
}

/// `minecraft:iron_golem`'s table is two unconditional pools the loot engine
/// evaluates whole: a poppy drawn 0..=2 and 3..=5 iron ingots
/// (`loot_table/entities/iron_golem.json`). `LivingEntity.dropFromLootTable`
/// spawns one item entity per drawn stack at
/// the mob's own position, and a stack drawn empty is refused by
/// `Entity.spawnAtLocation`.
#[test]
fn an_iron_golems_death_spawns_its_loot_table_stacks_at_its_position() {
    let mut world = spawn_pinned_world(datapack_config());
    let golem = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
    stage_one_blow_from_death(&mut world, golem);
    assert!(
        items(&world).is_empty(),
        "nothing was dropped before the kill"
    );

    kill_stored(&mut world, golem);

    let dropped = items(&world);
    assert!(
        dropped.len() == 1 || dropped.len() == 2,
        "the two pools draw a poppy and iron ingots, an empty draw refusing \
         its own spawn: {dropped:?}"
    );
    let mut expected = vec!["minecraft:iron_ingot".to_string()];
    if dropped.len() == 2 {
        expected.push("minecraft:poppy".to_string());
    }
    expected.sort();
    assert_eq!(dropped_ids(&world), expected);
    for item in &dropped {
        assert_eq!(item.1, MOB_POSITION);
        assert_eq!(
            item.4, 10,
            "`spawnAtLocation` sets the default pickup delay (`Entity.java:2137`)"
        );
    }
    let iron = dropped
        .iter()
        .find(|item| item.2 == "minecraft:iron_ingot")
        .expect("the iron pool always draws");
    assert!(
        (3..=5).contains(&iron.3),
        "the iron pool's set_count is uniform 3..=5: {iron:?}"
    );
    if let Some(poppy) = dropped.iter().find(|item| item.2 == "minecraft:poppy") {
        assert!((1..=2).contains(&poppy.3), "{poppy:?}");
    }
}

/// A saved mob keeps both `DeathLootTable` and `DeathLootTableSeed` through the
/// entity-region writer and loader; the restored death must spend that table
/// and draw from the seed ahead of the table's random sequence.
#[test]
fn a_saved_death_loot_override_draws_from_its_seed_after_reload() {
    let mut world = spawn_pinned_world(datapack_config());
    let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    load_record(
        &mut world,
        cow,
        vec![
            nbt_entry(
                "DeathLootTable",
                NbtSummaryValue::string("minecraft:entities/iron_golem"),
            ),
            nbt_entry("DeathLootTableSeed", NbtSummaryValue::Int(3)),
        ],
    );
    let uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
        .unwrap()
        .uuid();
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the entity region saves");
    assert!(!saved_entities(&request).is_empty());
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let mut restored = spawn_pinned_restore(datapack_config(), &candidate);
    let reloaded_cow = restored
        .live_entity_id_for_uuid_for_test(Java26BuiltinDimension::Overworld, uuid)
        .expect("the cow reloads");
    let record = command_record(&restored, uuid);
    assert_eq!(
        nbt_field(&record, "DeathLootTable"),
        &NbtSummaryValue::string("minecraft:entities/iron_golem")
    );
    assert_eq!(
        nbt_field(&record, "DeathLootTableSeed"),
        &NbtSummaryValue::Long(3)
    );

    stage_one_blow_from_death(&mut restored, reloaded_cow);
    kill_stored(&mut restored, reloaded_cow);

    let drawn: Vec<(String, i32)> = items(&restored)
        .into_iter()
        .map(|item| (item.2, item.3))
        .collect();
    // The iron golem's table, not the cow's: 0-2 poppies and 3-5 ingots.
    assert!(
        drawn.iter().any(|(id, _)| id == "minecraft:iron_ingot"),
        "{drawn:?}"
    );
    for (id, count) in &drawn {
        match id.as_str() {
            "minecraft:poppy" => assert!((1..=2).contains(count), "{drawn:?}"),
            "minecraft:iron_ingot" => assert!((3..=5).contains(count), "{drawn:?}"),
            _ => panic!("{drawn:?}"),
        }
    }
    assert!(command_record(&restored, uuid)
        .iter()
        .all(|entry| entry.name != "DeathLootTable"));
}

/// `Mob.dropCustomDeathLoot`: the gate is
/// `dropChance != 0`, `(killedByPlayer || preserve)` and one `this.random`
/// draw, so a chance of `1.0` still needs a player kill while a preserved
/// chance above `1.0` does not, and a zero chance never draws at all.
#[test]
fn equipment_drops_honor_the_drop_chance_and_the_preserved_gate() {
    for (chance, player_kill, expected) in [
        (1.0_f32, false, false),
        (1.0, true, true),
        (0.0, true, false),
        (2.0, false, true),
    ] {
        let (sender, receiver, mut world) = helpers::joined_stand_world(
            helpers::operator_datapack_config(),
            Vec3d {
                x: 2.5,
                y: 64.0,
                z: 0.5,
            },
        );
        let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        load_record(&mut world, zombie, drop_chances("mainhand", chance));
        equip(&mut world, zombie, "minecraft:diamond_sword");
        stage_one_blow_from_death(&mut world, zombie);

        if player_kill {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::Attack(AttackEntity { entity_id: zombie }),
            );
        } else {
            kill_stored(&mut world, zombie);
        }

        let swords = items(&world)
            .into_iter()
            .filter(|item| item.2 == "minecraft:diamond_sword")
            .count();
        assert_eq!(
            swords,
            usize::from(expected),
            "chance {chance}, player kill {player_kill}"
        );
    }
}

/// A cow at [`MOB_POSITION`], the wither that will take its kill credit, and
/// the sessions a chat command needs.
fn wither_kill_fixture() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    Uuid,
) {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = datapack_config().datapacks;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    // Out of the item-pickup range of the cow's own cell, so the rose item
    // the fallback arm adds stays on the ground for the assertion.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let cow = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    let (_wither, uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_WITHER_ID,
        Vec3d {
            x: 5.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(
            GeneralLivingData::new(VANILLA_ENTITY_TYPE_WITHER_ID).expect("a wither is living"),
        ),
    );
    (sender, receiver, world, cow, uuid)
}

/// `damage <target> <amount> <type> by <entity>` (`DamageCommand.java`), the
/// production entry that names a stored causing entity.
fn kill_by(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    cow: i32,
    wither_uuid: Uuid,
) {
    chat_command(
        sender,
        receiver,
        world,
        1,
        &format!("damage @e[type=minecraft:cow,limit=1] 40 minecraft:generic by {wither_uuid}"),
    );
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
            .and_then(crate::entity::Entity::living)
            .is_some_and(crate::entity::LivingEntityState::is_dead_or_dying),
        "the cow died to the wither's blow"
    );
}

/// `LivingEntity.createWitherRose` with a
/// `WitherBoss` kill credit: `MOB_GRIEFING` is on and the cell is air over a
/// support, so the rose is placed as a block while the cow's table evaluates.
#[test]
fn a_wither_kill_places_a_rose_on_the_mobs_cell() {
    let (sender, receiver, mut world, cow, wither) = wither_kill_fixture();
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: MOB_BLOCK.x,
            y: MOB_BLOCK.y - 1,
            z: MOB_BLOCK.z,
        },
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        live_block(&mut world, MOB_BLOCK),
        Some(block_state_id("minecraft:air"))
    );

    kill_by(&sender, &receiver, &mut world, cow, wither);

    assert_eq!(
        live_block(&mut world, MOB_BLOCK),
        Some(block_state_id("minecraft:wither_rose"))
    );
    assert!(
        !items(&world)
            .iter()
            .any(|item| item.2 == "minecraft:wither_rose"),
        "the block arm leaves no rose item: {:?}",
        items(&world)
    );
    assert!(
        orbs(&world).is_empty(),
        "no player's blow is remembered, so `dropExperience` (`:1498`) awards nothing"
    );
}

/// The same condition with a cell that cannot hold the rose: the item is added
/// with the bare `new ItemEntity` constructor, whose `pickupDelay` stays at
/// zero.
#[test]
fn a_wither_kill_falls_back_to_the_rose_item_off_a_bad_support() {
    let (sender, receiver, mut world, cow, wither) = wither_kill_fixture();
    // `WitherRoseBlock.mayPlaceOn` reads
    // `#minecraft:supports_wither_rose`, which glass is not in.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: MOB_BLOCK.x,
            y: MOB_BLOCK.y - 1,
            z: MOB_BLOCK.z,
        },
        block_state_id("minecraft:glass"),
        SET_BLOCK_UPDATE_ALL,
    ));

    kill_by(&sender, &receiver, &mut world, cow, wither);

    assert_eq!(
        live_block(&mut world, MOB_BLOCK),
        Some(block_state_id("minecraft:air"))
    );
    let roses: Vec<StoredItem> = items(&world)
        .into_iter()
        .filter(|item| item.2 == "minecraft:wither_rose")
        .collect();
    assert_eq!(roses.len(), 1, "{roses:?}");
    assert_eq!(roses[0].3, 1);
    assert_eq!(roses[0].4, 0);
}

/// `LivingEntity.die` reaches the transition once (`:1426-1457`), so a second
/// kill cannot draw the table again.
#[test]
fn a_second_kill_does_not_drop_the_loot_table_twice() {
    let mut world = spawn_pinned_world(datapack_config());
    let golem = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
    stage_one_blow_from_death(&mut world, golem);

    kill_stored(&mut world, golem);
    let after_first = items(&world);
    assert!(!after_first.is_empty(), "the first death dropped its table");

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            golem,
            "minecraft:generic",
            40.0,
            None,
        )
        .expect("the dying mob still answers the command");
    assert!(
        !outcome.succeeded,
        "a hurt on an already dying mob is refused"
    );
    assert_eq!(items(&world), after_first);
}

#[test]
fn an_enderman_drops_its_saved_carried_block_using_the_enchantment_provider() {
    let table = serde_json::json!({"type":"minecraft:block","pools":[
        {"rolls":1,"entries":[{"type":"minecraft:item","name":"minecraft:apple"}]},
        {"rolls":1,"conditions":[
            {"condition":"minecraft:match_tool","predicate":{"predicates":{
                "minecraft:enchantments":[{"enchantments":"minecraft:silk_touch","levels":{"min":1}}]
            }}},
            {"condition":"minecraft:location_check","offsetX":1,"predicate":{"block":{
                "nbt":"{id:'minecraft:chest',x:1,y:64,z:0,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"
            }}},
            {"condition":"minecraft:weather_check","raining":false}
        ],"entries":[{"type":"minecraft:item","name":"minecraft:glass"}]}
    ],"functions":[{"function":"minecraft:set_components","components":{"minecraft:max_stack_size":7}}]});
    let root = std::env::temp_dir().join(format!("bbb-enderman-block-weather-{}", Uuid::new_v4()));
    let pack = root.join("weather");
    std::fs::create_dir_all(pack.join("data/minecraft/loot_table/blocks")).unwrap();
    std::fs::write(pack.join("pack.mcmeta"), r#"{"pack":{"description":"Enderman carried block weather","pack_format":1000,"supported_formats":[0,10000]}}"#).unwrap();
    std::fs::write(
        pack.join("data/minecraft/loot_table/blocks/glass.json"),
        serde_json::to_vec(&table).unwrap(),
    )
    .unwrap();
    let startup = bbb_server_datapack::startup::DatapackStartup::load(
        &bbb_server_datapack::startup::DatapackStartupOptions {
            datapacks_dir: Some(root.clone()),
            saved_data_packs: None,
            safe_mode: false,
            limits: Default::default(),
        },
    )
    .unwrap();
    let config = ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..ServerWorldConfig::default()
    };
    std::fs::remove_dir_all(root).unwrap();
    for (carrying, raining) in [(false, false), (true, false), (true, true)] {
        let mut world = spawn_pinned_world(config.clone());
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut world, 1, "observer", 1);
        for (index, command) in [
            "setblock 1 64 0 chest",
            "item replace block 1 64 0 container.0 with diamond 3",
        ]
        .iter()
        .enumerate()
        {
            let result = console_command(&sender, &receiver, &mut world, index as u64 + 1, command);
            let answer = console_command_answer(&result);
            assert_eq!(answer.0, Some(1), "{command}: {answer:?}");
        }
        let command = if raining {
            "weather rain 100000t"
        } else {
            "weather clear 100000t"
        };
        let mut tick = console_command(&sender, &receiver, &mut world, 3, command);
        assert_eq!(console_command_answer(&tick).0, Some(100000));
        for _ in 0..200 {
            for event in &tick.events {
                if let WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::KeepAlive { id },
                } = event
                {
                    enqueue(
                        &sender,
                        WorldCommand::Play {
                            connection_id: *connection_id,
                            packet: PlayServerbound::KeepAlive { id: *id },
                        },
                    )
                    .unwrap();
                }
            }
            if world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining()
                == raining
            {
                break;
            }
            tick = world.tick(receiver.drain());
        }
        assert_eq!(
            world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining(),
            raining
        );
        let enderman = spawn_mob(
            &mut world,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        );
        if carrying {
            load_record(
                &mut world,
                enderman,
                vec![nbt_entry(
                    "carriedBlockState",
                    NbtSummaryValue::Compound(vec![nbt_entry(
                        "Name",
                        NbtSummaryValue::String("minecraft:glass".into()),
                    )]),
                )],
            );
        }
        let canonical = world
            .stored_entity_command_data_for_test(Java26BuiltinDimension::Overworld, enderman)
            .unwrap();
        let saved_carry = crate::loot_runtime::nbt_field(&canonical, "carriedBlockState");
        if carrying {
            assert_eq!(
                saved_carry.and_then(|state| crate::loot_runtime::nbt_field(state, "Name")),
                Some(&NbtSummaryValue::String("minecraft:glass".into()))
            );
        } else {
            assert!(saved_carry.is_none());
        }
        kill_stored(&mut world, enderman);
        let apples: Vec<_> = items(&world)
            .into_iter()
            .filter(|item| item.2 == "minecraft:apple")
            .collect();
        assert_eq!(apples.len(), usize::from(carrying));
        if carrying {
            assert_eq!(apples[0].3, 1);
        }
        let glass: Vec<_> = items(&world)
            .into_iter()
            .filter(|item| item.2 == "minecraft:glass")
            .collect();
        assert_eq!(glass.len(), usize::from(carrying && !raining));
        if carrying && !raining {
            assert_eq!(glass[0].1, MOB_POSITION);
            assert_eq!(glass[0].3, 1);
            assert_eq!(glass[0].4, 10);
            let EntityData::Item(item) = world
                .stored_entity_for_test(Java26BuiltinDimension::Overworld, glass[0].0)
                .unwrap()
                .data()
            else {
                unreachable!()
            };
            assert_eq!(item.item().max_stack_size().unwrap(), 7);
        }
        assert_eq!(
            world
                .level_weather(Java26BuiltinDimension::Overworld)
                .is_raining(),
            raining
        );
    }
}

/// A tame wolf's own bite blames its owner:
/// the skeleton it kills is Alice's kill and drops its experience, and the
/// helmet it is sure to drop (chance 2) still drops though the wolf is lifted
/// out of the store mid-bite.
#[test]
fn a_tame_wolfs_kill_is_its_owners_kill() {
    let table = serde_json::json!({"type":"minecraft:entity","pools":[{
        "rolls":1,"conditions":[{
            "condition":"minecraft:entity_properties","entity":"attacker",
            "predicate":{"type":"minecraft:wolf","team":"hunters"}
        }],"entries":[{"type":"minecraft:item","name":"minecraft:diamond"}]
    }]});
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks =
        datapack_config_with_loot_table("minecraft:entities/skeleton", &table).datapacks;
    let (sender, receiver, mut world) = helpers::joined_world(config);
    for command in ["team add hunters", "team join hunters alice"] {
        console_command(&sender, &receiver, &mut world, 30, command);
    }
    let skeleton = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SKELETON_ID);
    load_record(&mut world, skeleton, drop_chances("head", 2.0));
    stage_one_blow_from_death(&mut world, skeleton);
    // Shaded from the sun, which would otherwise burn the 0.1 health away.
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, skeleton)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(
            EquipmentSlot::Head,
            Some(crate::entity::item::plain_stack("minecraft:leather_helmet", 1).unwrap()),
        );
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let (_, wolf) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        VANILLA_ENTITY_TYPE_WOLF_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_WOLF_ID).unwrap()),
    );
    console_command(
        &sender,
        &receiver,
        &mut world,
        31,
        &format!("data merge entity {wolf} {{Owner:[I;0,0,0,1]}}"),
    );
    // A skeleton flees wolves; hold it still.
    let skeleton_uuid = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, skeleton)
        .unwrap()
        .uuid();
    console_command(
        &sender,
        &receiver,
        &mut world,
        32,
        &format!("data merge entity {skeleton_uuid} {{NoAI:1b}}"),
    );
    for _ in 0..200 {
        world.tick(receiver.drain());
        if world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, skeleton)
            .is_none_or(|skeleton| skeleton.living().unwrap().is_dead_or_dying())
        {
            break;
        }
    }
    let skeleton = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, skeleton)
        .expect("the dying skeleton is still stored");
    assert!(skeleton.living().unwrap().is_dead_or_dying());
    assert_eq!(
        skeleton.living().unwrap().last_hurt_by_player,
        Some(Uuid::from_u128(1))
    );
    let experience: i32 = orbs(&world)
        .iter()
        .map(|(_, value, count)| value * count)
        .sum();
    // A preserved helmet adds nothing to `xpReward`.
    assert_eq!(experience, 5, "a player's kill awards experience");
    assert_eq!(
        dropped_ids(&world),
        ["minecraft:diamond", "minecraft:leather_helmet"],
        "the biting wolf inherits its owner's team while detached from the store"
    );
}

/// A lingering cloud Alice owns kills a cow with its harming and commits the
/// death once: one stack of beef and the cow's 1..=3 experience for her kill.
#[test]
fn a_lethal_cloud_drops_the_death_loot_once() {
    let (sender, receiver, mut world, cow) = player_kill_fixture(VANILLA_ENTITY_TYPE_COW_ID, None);
    // Out of pickup reach of the drops.
    stand_at(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let potion_contents = bbb_protocol::packets::PotionContentsSummary {
        custom_effects: vec![bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID,
            amplifier: 1,
            duration: 0,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }],
        ..Default::default()
    };
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        MOB_POSITION,
        EntityData::AreaEffectCloud(
            crate::entity::area_effect_cloud::AreaEffectCloudData::new()
                .with_wait_time(0)
                .with_radius(1.0)
                .with_duration(200)
                .with_owner(Some(Uuid::from_u128(1)))
                .with_potion_contents(Some(potion_contents)),
        ),
    );
    for _ in 0..30 {
        world.tick(receiver.drain());
    }
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, cow)
        .is_none_or(|cow| cow.living().unwrap().is_dead_or_dying()));
    let beef = items(&world)
        .into_iter()
        .filter(|item| item.2 == "minecraft:beef")
        .count();
    assert_eq!(beef, 1, "{:?}", items(&world));
    let experience: i32 = orbs(&world)
        .iter()
        .map(|(_, value, count)| value * count)
        .sum();
    assert!((1..=3).contains(&experience), "{:?}", orbs(&world));
}

/// `Mob.dropCustomDeathLoot`'s damage arm: a dropped
/// damageable item that is not preserved leaves with a random damage value
/// drawn from the mob's own random, `maxDamage - nextInt(1 + nextInt(
/// max(maxDamage - 3, 1)))`, which is never zero and never the full bar.
#[test]
fn a_dropped_equipment_item_leaves_with_a_random_damage_value() {
    let (sender, receiver, mut world) = helpers::joined_stand_world(
        helpers::operator_datapack_config(),
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    load_record(&mut world, zombie, drop_chances("mainhand", 1.0));
    equip(&mut world, zombie, "minecraft:diamond_sword");
    stage_one_blow_from_death(&mut world, zombie);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: zombie }),
    );

    let sword = items(&world)
        .into_iter()
        .find(|item| item.2 == "minecraft:diamond_sword")
        .expect("a guaranteed chance and a player kill drop the sword");
    let damage = sword
        .5
        .expect("the dropped sword carries the damage component");
    assert!(damage > 0, "the damage arm writes a positive value");
}

#[test]
fn the_killers_luck_is_the_session_luck_attribute_the_death_loot_reads() {
    let table = serde_json::json!({"type":"minecraft:entity","pools":[{
        "rolls":1, "conditions":[{"condition":"minecraft:killed_by_player"},
            {"condition":"minecraft:entity_properties","entity":"attacking_player","predicate":{"type":"minecraft:player","team":"hunters","movement":{"speed":{"max":1000}}}}],
        "entries":[{"type":"minecraft:item","name":"minecraft:player_head","functions":[
            {"function":"minecraft:copy_name","source":"this"},
            {"function":"minecraft:fill_player_head","entity":"attacking_player"},
            {"function":"minecraft:copy_custom_data","source":"attacking_player","ops":[
                {"source":"Dimension","target":"dimension","op":"replace"},
                {"source":"SelectedItem.id","target":"selected","op":"replace"},
                {"source":"RootVehicle.Entity.id","target":"vehicle","op":"replace"},
                {"source":"warden_spawn_tracker","target":"tracker","op":"replace"}
            ]}
        ]}]
    }]});
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = datapack_config_with_loot_table("minecraft:entities/cow", &table).datapacks;
    let (sender, receiver, mut world) = helpers::joined_world(config);
    for command in ["team add hunters", "team join hunters alice"] {
        console_command(&sender, &receiver, &mut world, 99, command);
    }
    let mob = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    let mut events = Vec::new();
    // `LuckMobEffect` adds one `minecraft:luck` per level
    // (`MobEffectInstance` scales the modifier by `amplifier + 1`).
    let luck_two =
        crate::entity::living::mob_effects::ActiveMobEffect::new("minecraft:luck", 200, 1)
            .expect("a vanilla effect id");
    assert_eq!(
        world.add_player_mob_effect(ConnectionId(1), luck_two, &mut events),
        Some(true)
    );
    let players = world.nearby_players_for_test(Java26BuiltinDimension::Overworld);
    let killer = players
        .first()
        .expect("the joined session is the only player in this level")
        .uuid;
    assert_eq!(
        players.first().map(|player| player.luck),
        Some(2.0),
        "the projection carries the attribute the effect modified"
    );
    assert_eq!(
        world.killer_luck_for_test(Java26BuiltinDimension::Overworld, Some(killer)),
        Some(2.0),
        "luck II is the base 0 plus the effect's two"
    );
    assert_eq!(
        world.killer_luck_for_test(Java26BuiltinDimension::Overworld, None),
        None,
        "no killer has no luck to read"
    );
    assert_eq!(
        world.killer_luck_for_test(
            Java26BuiltinDimension::Overworld,
            Some(Uuid::from_u128(0xfeed))
        ),
        None,
        "a killer no session holds is the null player"
    );
    load_record(
        &mut world,
        mob,
        vec![
            nbt_entry("last_hurt_by_player", crate::tests::nbt_uuid(killer)),
            nbt_entry("last_hurt_by_player_memory_time", NbtSummaryValue::Int(100)),
            nbt_entry("CustomName", NbtSummaryValue::String("Victim".into())),
        ],
    );
    console_command(
        &sender,
        &receiver,
        &mut world,
        99,
        "execute in minecraft:the_nether run tp alice 2.5 64 0.5",
    );
    assert!(world
        .nearby_players_for_test(Java26BuiltinDimension::Overworld)
        .is_empty());
    assert_eq!(
        world.killer_luck_for_test(Java26BuiltinDimension::Overworld, Some(killer)),
        Some(2.0)
    );
    console_command(
        &sender,
        &receiver,
        &mut world,
        99,
        "give alice minecraft:diamond_sword",
    );
    let (vehicle, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Nether,
        VANILLA_ENTITY_TYPE_COW_ID,
        MOB_POSITION,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
    );
    assert!(world.attach_external_player_for_test(ConnectionId(1), vehicle));
    let player_data = world.command_entity_data_snapshot_for_test();
    let player_data = &player_data
        .iter()
        .find(|entry| entry.uuid == killer)
        .unwrap()
        .data;
    kill_stored(&mut world, mob);
    let drops = items(&world);
    let head = drops
        .iter()
        .find(|item| item.2 == "minecraft:player_head")
        .unwrap();
    let stack = world
        .stored_item_stack_for_test(Java26BuiltinDimension::Overworld, head.0)
        .unwrap();
    let component = |id| crate::tests::saved_item_component(&world, &stack, id);
    assert_eq!(
        component("minecraft:custom_name"),
        Some(NbtSummaryValue::String("Victim".into()))
    );
    use crate::loot_runtime::{nbt_field, nbt_string};
    assert_eq!(
        nbt_string(&component("minecraft:profile").unwrap(), "name").as_deref(),
        Some("alice")
    );
    let copied = component("minecraft:custom_data").unwrap();
    assert_eq!(
        nbt_string(&copied, "dimension").as_deref(),
        Some("minecraft:the_nether")
    );
    assert_eq!(
        nbt_string(&copied, "selected").as_deref(),
        Some("minecraft:diamond_sword")
    );
    assert_eq!(
        nbt_string(&copied, "vehicle").as_deref(),
        Some("minecraft:cow")
    );
    assert_eq!(
        nbt_field(&copied, "tracker"),
        nbt_field(player_data, "warden_spawn_tracker")
    );
    world.load_stored_entity_record_for_test(
        Java26BuiltinDimension::Nether,
        vehicle,
        &[
            nbt_entry("last_hurt_by_player", crate::tests::nbt_uuid(killer)),
            nbt_entry("last_hurt_by_player_memory_time", NbtSummaryValue::Int(100)),
        ],
    );
    assert!(
        world
            .hurt_stored_entity_by_command_for_test(
                Java26BuiltinDimension::Nether,
                vehicle,
                "minecraft:generic",
                40.0,
                None,
            )
            .unwrap()
            .succeeded
    );
    assert_eq!(
        world
            .stored_items_for_test(Java26BuiltinDimension::Nether)
            .iter()
            .filter(|item| item.2 == "minecraft:player_head")
            .count(),
        1,
    );
    enqueue(
        &sender,
        WorldCommand::Disconnect {
            connection_id: ConnectionId(1),
        },
    )
    .unwrap();
    world.tick(receiver.drain());
    assert_eq!(
        world.killer_luck_for_test(Java26BuiltinDimension::Overworld, Some(killer)),
        None
    );
    let mob = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    load_record(
        &mut world,
        mob,
        vec![
            nbt_entry("last_hurt_by_player", crate::tests::nbt_uuid(killer)),
            nbt_entry("last_hurt_by_player_memory_time", NbtSummaryValue::Int(100)),
        ],
    );
    kill_stored(&mut world, mob);
    assert_eq!(
        items(&world)
            .iter()
            .filter(|item| item.2 == "minecraft:player_head")
            .count(),
        1
    );
}

#[test]
fn a_rear_passenger_killers_death_loot_reads_the_boat_controllers_actual_movement() {
    let pools: Vec<_> = [
        (
            "minecraft:amethyst_shard",
            serde_json::json!({"min":4,"max":6}),
        ),
        ("minecraft:diamond", serde_json::json!({"max":1})),
    ]
    .into_iter()
    .map(|(item, x)| {
        serde_json::json!({
            "rolls":1, "entries":[{"type":"minecraft:item","name":item}],
            "conditions":[{"condition":"minecraft:entity_properties","entity":"attacking_player",
                "predicate":{"movement":{"x":x}}}]
        })
    })
    .collect();
    let table = serde_json::json!({"type":"minecraft:entity","pools":pools});
    let mut world = spawn_pinned_world(datapack_config_with_loot_table(
        "minecraft:entities/cow",
        &table,
    ));
    let (sender, receiver) = mailbox();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let start = Vec3d {
        x: 0.5,
        y: 300.0,
        z: 0.5,
    };
    for connection in [ConnectionId(1), ConnectionId(2)] {
        world.set_player_position_for_test(connection, start);
    }
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(Vec3d { x: 0.75, ..start }),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    let dimension = Java26BuiltinDimension::Overworld;
    let (boat, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
        start,
        EntityData::Boat(crate::entity::BoatData::new()),
    );
    assert!(world.attach_external_player_for_test(ConnectionId(1), boat));
    assert!(world.attach_external_player_for_test(ConnectionId(2), boat));
    let killer = world
        .nearby_players_for_test(dimension)
        .into_iter()
        .find(|player| player.entity_id == world.player_entity_id_for_test(ConnectionId(2)))
        .unwrap()
        .uuid;
    let mob = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_COW_ID);
    load_record(
        &mut world,
        mob,
        vec![
            nbt_entry("last_hurt_by_player", crate::tests::nbt_uuid(killer)),
            nbt_entry("last_hurt_by_player_memory_time", NbtSummaryValue::Int(100)),
        ],
    );
    kill_stored(&mut world, mob);
    assert_eq!(dropped_ids(&world), ["minecraft:amethyst_shard"]);
}

/// The stored source a `/damage ... by <stored entity>` names.
fn stored_damage_source(
    uuid: Uuid,
    entity_id: i32,
    resource_id: &str,
    is_mob: bool,
) -> crate::commands::CommandDamageSourceEntity {
    helpers::stored_damage_source(uuid, entity_id, resource_id, is_mob, MOB_POSITION)
}

/// One `minecraft:entities/*` table whose terms include one this engine cannot
/// answer is refused whole and reported, while the equipment drop beside it
/// still happens: `dropCustomDeathLoot` is a separate call in
/// `dropAllDeathLoot`.
///
/// `magma_cube`'s three froglight entries read
/// `source_entity.components.minecraft:frog/variant`; a source entity with no
/// component map — an armor stand, the one non-mob living type this crate
/// stores — leaves that read unanswerable, so the term is a counted refusal
/// rather than a false the source's own `type` already answered for.
#[test]
fn a_refused_loot_table_is_reported_and_equipment_still_drops() {
    let mut world = spawn_pinned_world(datapack_config());
    let magma_cube = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID);
    load_record(&mut world, magma_cube, drop_chances("mainhand", 2.0));
    equip(&mut world, magma_cube, "minecraft:diamond_sword");
    let (stand, stand_uuid) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
        EntityData::ArmorStand(
            crate::entity::armor_stand::ArmorStandData::new()
                .expect("the armor stand has attributes"),
        ),
    );
    // Loading the record restores the mob's own health, so the blow that has to
    // be lethal is staged after it.
    stage_one_blow_from_death(&mut world, magma_cube);

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            Java26BuiltinDimension::Overworld,
            magma_cube,
            "minecraft:generic",
            40.0,
            Some(stored_damage_source(
                stand_uuid,
                stand,
                "minecraft:armor_stand",
                false,
            )),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);

    assert_eq!(
        dropped_ids(&world),
        vec!["minecraft:diamond_sword".to_string()]
    );
}

/// The row's last item: `/summon` runs the `finalizeSpawn` vanilla runs, so a
/// summoned frog carries the variant its own finalizer selected and
/// `minecraft:frog/variant` answers for the table that reads it
///
/// Every corner the fuzzy biome zoom can answer from is desert, so the variant
/// the summon draws is the warm one rather than the temperate default the
/// constructor seeds — the froglight this kill leaves is the drawn variant's,
/// and the whole table evaluates, which is what it could not do before this
/// finalize existed.
#[test]
fn a_summoned_frog_carries_the_variant_its_loot_term_reads() {
    let dimension = Java26BuiltinDimension::Overworld;
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    // `fuzzy_biome_quart_position(seed, 2, 64, 0)` is one of the eight corners
    // of quart (0, 15, -1), and `minecraft:desert` is in
    // `#minecraft:spawns_warm_variant_frogs`.
    for quart_x in 0..=1 {
        for quart_y in 15..=16 {
            for quart_z in -1..=0 {
                assert!(world.set_noise_biome_for_test(
                    dimension,
                    (quart_x, quart_y, quart_z),
                    "minecraft:desert"
                ));
            }
        }
    }
    let summoned = console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "summon minecraft:frog 2.5 64 0.5",
    );
    assert_eq!(
        console_command_answer(&summoned),
        (Some(1), vec!["commands.summon.success".to_owned()])
    );
    let ids = world.stored_entity_ids_for_test(dimension);
    let [frog] = ids.as_slice() else {
        panic!("one summoned frog, not {ids:?}")
    };
    let frog = *frog;
    let frog_uuid = world
        .stored_entity_for_test(dimension, frog)
        .expect("the summoned frog is filed in the level")
        .uuid();
    let record = command_record(&world, frog_uuid);
    assert_eq!(
        nbt_field(&record, "variant"),
        &NbtSummaryValue::string("minecraft:warm")
    );

    let magma_cube = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID);
    load_record(&mut world, magma_cube, drop_chances("mainhand", 2.0));
    equip(&mut world, magma_cube, "minecraft:diamond_sword");
    stage_one_blow_from_death(&mut world, magma_cube);

    let outcome = world
        .hurt_stored_entity_by_command_for_test(
            dimension,
            magma_cube,
            "minecraft:generic",
            40.0,
            Some(stored_damage_source(
                frog_uuid,
                frog,
                "minecraft:frog",
                true,
            )),
        )
        .expect("the fixture mob takes command damage");
    assert!(outcome.applied && outcome.succeeded);

    // The magma_cream pool is the inverted frog source, so the frog's own blow
    // leaves the drawn variant's froglight and the forced equipment drop.
    assert_eq!(
        dropped_ids(&world),
        vec![
            "minecraft:diamond_sword".to_string(),
            "minecraft:pearlescent_froglight".to_string()
        ]
    );
}

/// One stored entity's own record, the shape `/data get entity` projects.
fn command_record(world: &ServerWorld, uuid: Uuid) -> Vec<NbtSummaryEntry> {
    let snapshot = world.command_entity_data_snapshot_for_test();
    let value = &snapshot
        .iter()
        .find(|entry| entry.uuid == uuid)
        .expect("the entity's command projection")
        .data;
    let NbtSummaryValue::Compound(entries) = value else {
        panic!("a projected entity is a compound: {value:?}");
    };
    entries.clone()
}

fn nbt_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> &'a NbtSummaryValue {
    &entries
        .iter()
        .find(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("the record writes {name}"))
        .value
}

/// Evaluates both the zombie's entity loot table and guaranteed equipment drops.
#[test]
fn an_evaluated_loot_table_still_drops_beside_the_equipment_drop() {
    let mut world = spawn_pinned_world(datapack_config());
    let zombie = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    load_record(&mut world, zombie, drop_chances("mainhand", 2.0));
    equip(&mut world, zombie, "minecraft:diamond_sword");
    stage_one_blow_from_death(&mut world, zombie);

    kill_stored(&mut world, zombie);

    // `/kill` names no attacker, so `killed_by_player` is false, looting is 0,
    // and the table's own first pool draws 0 to 2 rotten flesh.
    let ids = dropped_ids(&world);
    assert!(
        ids.contains(&"minecraft:diamond_sword".to_string()),
        "{ids:?}"
    );
    assert!(
        ids.iter()
            .all(|id| id == "minecraft:diamond_sword" || id == "minecraft:rotten_flesh"),
        "{ids:?}"
    );
}

/// A joined session standing two blocks off the mob's own cell, holding
/// `mainhand`, with the mob staged one blow from death.
fn player_kill_fixture(
    entity_type_id: i32,
    mainhand: Option<(&str, &[(&str, i32)])>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    player_kill_fixture_in(datapack_config(), entity_type_id, mainhand)
}

fn player_kill_fixture_in(
    datapacks: ServerWorldConfig,
    entity_type_id: i32,
    mainhand: Option<(&str, &[(&str, i32)])>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = datapacks.datapacks;
    let (sender, receiver, mut world) = helpers::joined_world(config);
    // Put there rather than walked there: a walk over the floor plays steps,
    // whose sound seeds would shift the level random the loot rolls read.
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    world.tick(receiver.drain());
    if let Some((item, enchantments)) = mainhand {
        let mut stack = plain_summary(item, 1);
        enchant(&world, &mut stack, enchantments);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, stack);
    }
    let mob = spawn_mob(&mut world, entity_type_id);
    stage_one_blow_from_death(&mut world, mob);
    (sender, receiver, world, mob)
}

/// One `ItemStackSummary` with no component patch beyond what `enchant` adds.
fn plain_summary(item: &str, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(
            bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
                .unwrap_or_else(|| panic!("{item} is a builtin item"))
                .id,
        ),
        count,
        ..ItemStackSummary::empty()
    }
}

/// The lethal hit a player's own swing is: `ServerGamePacketListenerImpl`
/// routes `ServerboundInteractPacket` into `Player.attack`
/// which is the damage source the loot context's
/// `ATTACKING_ENTITY` then names.
fn attack_by_player(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    mob: i32,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: mob }),
    );
    assert!(
        world
            .stored_entity_for_test(Java26BuiltinDimension::Overworld, mob)
            .and_then(crate::entity::Entity::living)
            .is_some_and(crate::entity::LivingEntityState::is_dead_or_dying),
        "the player's swing was lethal"
    );
}

/// Puts `holder_id`-resolved enchantments on a stack the way a decoded
/// `minecraft:enchantments` component carries them.
fn enchant(world: &ServerWorld, tool: &mut ItemStackSummary, enchantments: &[(&str, i32)]) {
    let registry: Vec<String> = world
        .datapacks()
        .expect("the fixture world has a datapack generation")
        .registries
        .registry("minecraft:enchantment")
        .expect("the generation carries the enchantment registry")
        .elements
        .iter()
        .map(|element| element.id.clone())
        .collect();
    for (id, level) in enchantments {
        tool.component_patch
            .enchantments
            .push(bbb_protocol::packets::ItemEnchantmentSummary {
                holder_id: i32::try_from(
                    registry
                        .iter()
                        .position(|entry| entry == id)
                        .unwrap_or_else(|| panic!("the loaded registry carries {id}")),
                )
                .unwrap(),
                level: *level,
            });
    }
    tool.component_patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID);
}

/// Every stack the mob's death left behind, as `(item, count)` sorted, which is
/// what `dropFromLootTable`'s one `spawnAtLocation` per drawn stack adds up to.
fn dropped_stacks(world: &ServerWorld) -> Vec<(String, i32)> {
    let mut stacks: Vec<(String, i32)> = items(world)
        .into_iter()
        .map(|item| (item.2, item.3))
        .collect();
    stacks.sort();
    stacks
}

/// A player's kill of a mob drops its built-in `minecraft:entities/<mob>`
/// table's own stacks. The killer holds a looting III sword, which is what the
/// `enchanted_count_increase` every one of these tables carries grows from.
#[test]
fn a_players_kill_drops_every_evaluated_mob_tables_own_items() {
    for (entity_type_id, table, ranges) in [
        (
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            "minecraft:entities/zombie",
            &[("minecraft:rotten_flesh", 0, 5)][..],
        ),
        (
            VANILLA_ENTITY_TYPE_SPIDER_ID,
            "minecraft:entities/spider",
            &[("minecraft:string", 0, 5), ("minecraft:spider_eye", -1, 4)][..],
        ),
        (
            VANILLA_ENTITY_TYPE_COW_ID,
            "minecraft:entities/cow",
            &[("minecraft:leather", 0, 5), ("minecraft:beef", 1, 6)][..],
        ),
    ] {
        let (sender, receiver, mut world, mob) = player_kill_fixture(
            entity_type_id,
            Some(("minecraft:diamond_sword", &[("minecraft:looting", 3)])),
        );
        attack_by_player(&sender, &receiver, &mut world, mob);
        let dropped = dropped_stacks(&world);
        // Each stack is one the table names, drawn inside that entry's own
        // `set_count` plus the looting III addition the same table carries.
        for (item, count) in &dropped {
            let (_, min, max) = ranges
                .iter()
                .find(|(name, _, _)| name == item)
                .unwrap_or_else(|| panic!("{table} drops {item}: {dropped:?}"));
            assert!(
                (*min..=*max).contains(count),
                "{table} draws {item} in {min}..={max}, not {count}"
            );
        }
    }
}

#[test]
fn the_looting_level_of_the_killers_sword_grows_the_counts_these_tables_set() {
    let (sender, receiver, mut world, mut mob) = player_kill_fixture(
        VANILLA_ENTITY_TYPE_SKELETON_ID,
        Some(("minecraft:diamond_sword", &[("minecraft:looting", 3)])),
    );

    // `skeleton.json` caps both of its `set_count`s at 2, so a bone stack of
    // more than 2 can only come from `enchanted_count_increase`
    // reading level 3 off the
    // killer's own main hand. Half of all kills draw one, so a few kills in a
    // row show it whatever the random stream.
    let mut bones = Vec::new();
    for _ in 0..12 {
        attack_by_player(&sender, &receiver, &mut world, mob);
        let bone = dropped_stacks(&world)
            .into_iter()
            .find(|(item, _)| item == "minecraft:bone")
            .map_or(0, |(_, count)| count);
        assert!(bone <= 5, "base 0..=2 plus looting 0..=3: {bone}");
        bones.push(bone);
        if bone > 2 {
            break;
        }
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "kill @e[type=minecraft:item]",
        );
        mob = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SKELETON_ID);
        stage_one_blow_from_death(&mut world, mob);
    }
    assert!(
        bones.iter().any(|&bone| bone > 2),
        "looting III adds round(3 * count) to the base draw: {bones:?}"
    );
}

/// `SmeltItemFunction.run` on the death path:
/// a cow that was on fire drops the smelting result of its own drop, which is
/// the item environment's `RecipeManager.getRecipeFor(RecipeType.SMELTING, …)`
#[test]
fn a_burning_cows_death_smelts_its_own_drop() {
    let (sender, receiver, mut world, mob) = player_kill_fixture(VANILLA_ENTITY_TYPE_COW_ID, None);
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, mob)
        .expect("the cow is stored")
        .set_remaining_fire_ticks(100);

    attack_by_player(&sender, &receiver, &mut world, mob);

    assert!(
        dropped_stacks(&world)
            .iter()
            .any(|(item, _)| item == "minecraft:cooked_beef"),
        "the burning cow's beef was smelted: {:?}",
        dropped_stacks(&world)
    );
}

/// The other arm of the same `any_of`: a killer holding a weapon enchanted with
/// a `#minecraft:smelts_loot` enchantment smelts the drop too, read through the
/// `entity_properties` `equipment.mainhand` term.
#[test]
fn a_fire_aspect_sword_smelts_the_drop_it_takes() {
    let (sender, receiver, mut world, mob) = player_kill_fixture(
        VANILLA_ENTITY_TYPE_COW_ID,
        Some(("minecraft:diamond_sword", &[("minecraft:fire_aspect", 1)])),
    );

    attack_by_player(&sender, &receiver, &mut world, mob);

    assert!(
        dropped_stacks(&world)
            .iter()
            .any(|(item, _)| item == "minecraft:cooked_beef"),
        "fire aspect smelts the beef: {:?}",
        dropped_stacks(&world)
    );
}

/// The potion a dropped item entity's `minecraft:potion_contents` names, read
/// off the component `set_potion` writes.
fn dropped_potion(world: &ServerWorld, entity_id: i32) -> Option<String> {
    let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)?;
    let EntityData::Item(item) = entity.data() else {
        return None;
    };
    let NbtSummaryValue::Compound(entries) =
        crate::tests::saved_item_component(world, item.item(), "minecraft:potion_contents")?
    else {
        return None;
    };
    entries
        .iter()
        .find(|entry| entry.name == "potion")
        .and_then(|entry| match &entry.value {
            NbtSummaryValue::String(potion) => Some(potion.try_to_string().unwrap()),
            _ => None,
        })
}

/// The drop sets of `minecraft:damage_source_properties` tables and the
/// pillager's `type_specific.raider` pool, as one player melee kill each with
/// a looting III sword. The `forbidden` lists are the entries a player's melee
/// kill cannot satisfy.
#[test]
fn a_players_kill_evaluates_the_damage_source_potion_and_raider_tables() {
    // `magma_cube.json` spawns at `Size: 1`, the zero-based size 2 its
    // `type_specific.slime` `size: {min: 2}` entry needs to be live.
    let slime_size_two = vec![nbt_entry("Size", NbtSummaryValue::Int(1))];
    for (entity_type_id, table, expected, forbidden, record) in [
        (
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
            "minecraft:entities/magma_cube",
            &[("minecraft:magma_cream", 3)][..],
            // The three froglight entries read a `minecraft:frog`
            // `source_entity`; a melee blow's source entity is the player.
            &[
                "minecraft:pearlescent_froglight",
                "minecraft:verdant_froglight",
                "minecraft:ochre_froglight",
            ][..],
            Some(slime_size_two),
        ),
        (
            VANILLA_ENTITY_TYPE_GHAST_ID,
            "minecraft:entities/ghast",
            &[("minecraft:ghast_tear", 3), ("minecraft:gunpowder", 5)][..],
            // Its third pool wants a `minecraft:fireball` `direct_entity` in
            // `#minecraft:is_projectile`; a melee blow's direct entity is the
            // player.
            &["minecraft:music_disc_tears"][..],
            None,
        ),
        (
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            "minecraft:entities/turtle",
            &[("minecraft:seagrass", 3)][..],
            // Its second pool wants `#minecraft:is_lightning`.
            &["minecraft:bowl"][..],
            None,
        ),
        (
            VANILLA_ENTITY_TYPE_PILLAGER_ID,
            "minecraft:entities/pillager",
            // Its only pool is gated on `is_captain`, and a summon without the
            // patrol-leader banner is not one.
            &[][..],
            &["minecraft:ominous_bottle"][..],
            None,
        ),
    ] {
        let (sender, receiver, mut world, mob) = player_kill_fixture(
            entity_type_id,
            Some(("minecraft:diamond_sword", &[("minecraft:looting", 3)])),
        );
        if let Some(entries) = record {
            load_record(&mut world, mob, entries);
            // Loading a record restores the mob's own health, so the blow that
            // has to be lethal is staged after it.
            stage_one_blow_from_death(&mut world, mob);
        }

        attack_by_player(&sender, &receiver, &mut world, mob);

        let dropped = dropped_stacks(&world);
        assert_eq!(
            dropped
                .iter()
                .map(|(item, _)| item.as_str())
                .collect::<Vec<_>>(),
            expected.iter().map(|(item, _)| *item).collect::<Vec<_>>(),
            "{table}"
        );
        for item in forbidden {
            assert!(
                !dropped.iter().any(|(name, _)| name == item),
                "{table} cannot draw {item} off a player's melee kill: {dropped:?}"
            );
        }
    }
}

/// The three `set_potion` tables: the tipped arrow a player's kill draws
/// carries the potion its own table names, written as the
/// `minecraft:potion_contents` component `SetPotionFunction.run`
/// sets.
#[test]
fn a_players_kill_sets_the_potion_each_of_these_tables_names() {
    for (entity_type_id, table, potion) in [
        (
            VANILLA_ENTITY_TYPE_STRAY_ID,
            "minecraft:entities/stray",
            "minecraft:slowness",
        ),
        (
            VANILLA_ENTITY_TYPE_PARCHED_ID,
            "minecraft:entities/parched",
            "minecraft:weakness",
        ),
        (
            VANILLA_ENTITY_TYPE_BOGGED_ID,
            "minecraft:entities/bogged",
            "minecraft:poison",
        ),
    ] {
        let (sender, receiver, mut world, mob) = player_kill_fixture(
            entity_type_id,
            Some(("minecraft:diamond_sword", &[("minecraft:looting", 3)])),
        );

        attack_by_player(&sender, &receiver, &mut world, mob);

        // `set_count` draws the tipped arrow's own count from `uniform 0..=1`,
        // so the pool may draw nothing at all under the pinned sequence; what
        // it can never draw is an arrow with another table's potion.
        for (entity_id, _, item, count, _, _) in items(&world) {
            if item != "minecraft:tipped_arrow" {
                continue;
            }
            assert_eq!(count, 1, "{table} sets the potion on one arrow");
            assert_eq!(
                dropped_potion(&world, entity_id).as_deref(),
                Some(potion),
                "{table} names {potion}"
            );
        }
    }
}

const BLAZE_XP_REWARD: i32 = 10;

/// `ExperienceOrb.awardWithDirection`'s split:
/// `getExperienceValue` cuts the amount down one orb at a time.
fn split_of(amount: i32) -> Vec<i32> {
    let mut values = Vec::new();
    let mut remaining = amount;
    while remaining > 0 {
        let step = bbb_server_types::experience::experience_value(remaining);
        remaining -= step;
        values.push(step);
    }
    values
}

/// One stored orb as the tests here read it: `(position, value, count)`.
fn orbs(world: &ServerWorld) -> Vec<(Vec3d, i32, i32)> {
    world
        .stored_experience_orbs_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .map(|(_, position, value, count)| (position, value, count))
        .collect()
}

/// `LivingEntity.dropExperience` through a real
/// player kill: the blow remembers itself for a hundred ticks,
/// which is the `lastHurtByPlayerMemoryTime > 0` arm, `shouldDropExperience`
/// and `MOB_DROPS` all open.
///
/// The amount is the type's declared `xpReward` (`Mob.getBaseExperienceReward`,
/// `Mob`) and the orbs are `award`'s split of it at the death
/// position.
#[test]
fn a_players_kill_awards_the_mobs_declared_experience_as_orbs() {
    let declared = BLAZE_XP_REWARD;
    let (sender, receiver, mut world, blaze) =
        player_kill_fixture(VANILLA_ENTITY_TYPE_BLAZE_ID, None);
    assert!(
        orbs(&world).is_empty(),
        "nothing is awarded before the kill: {:?}",
        orbs(&world)
    );

    attack_by_player(&sender, &receiver, &mut world, blaze);

    let awarded = orbs(&world);
    assert_eq!(
        awarded
            .iter()
            .map(|(_, value, _)| *value)
            .collect::<Vec<_>>(),
        split_of(declared),
        "the declared reward {declared} is awarded as `getExperienceValue`'s orbs"
    );
    for (position, value, count) in &awarded {
        assert_eq!(*count, 1, "one award per orb, value {value}");
        // `ExperienceOrb.award` spawns at `this.position()` (`:1500`) and the
        // constructor's own draw bounds one step of movement at 0.2 on x and z
        // and 0.4 upwards (`:56-58`); `play` ticks the world once, so the orbs
        // here have already taken that step. The exact spawn geometry is pinned
        // in `a_death_awards_the_declared_reward_as_split_orbs_after_its_loot`.
        let offset = Vec3d {
            x: position.x - MOB_POSITION.x,
            y: position.y - MOB_POSITION.y,
            z: position.z - MOB_POSITION.z,
        };
        assert!(
            offset.x.abs() < 0.2 && offset.y > 0.0 && offset.y < 0.4 && offset.z.abs() < 0.2,
            "orb {value} left the death position by one constructor step: {offset:?}"
        );
    }
}

/// `!this.wasExperienceConsumed()`: the flag
/// `SculkCatalystBlockEntity.handleGameEvent` sets on the mob it consumed
/// removes the award a player kill
/// would otherwise make, and nothing else about the death.
#[test]
fn a_mob_whose_experience_was_consumed_awards_nothing() {
    let declared = BLAZE_XP_REWARD;
    for consumed in [false, true] {
        let (sender, receiver, mut world, blaze) =
            player_kill_fixture(VANILLA_ENTITY_TYPE_BLAZE_ID, None);
        if consumed {
            world
                .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, blaze)
                .expect("the blaze is stored")
                .living_mut()
                .expect("a blaze is living")
                .skip_drop_experience();
        }

        attack_by_player(&sender, &receiver, &mut world, blaze);

        let total: i32 = orbs(&world)
            .iter()
            .map(|(_, value, count)| value * count)
            .sum();
        assert_eq!(
            total,
            if consumed { 0 } else { declared },
            "consumed {consumed}"
        );
    }
}

/// A patrol leader pillager — the captain `Raider.isCaptain`
/// is true of, because `PatrolSpawner`'s own finalize set `PatrolLeader` and
/// equipped `Raid.getOminousBannerInstance` — staged one blow from death, with
/// the floor `NaturalSpawner.isSpawnPositionOk`'s `ON_GROUND` arm reads.
fn patrol_leader_kill_fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver, mut world) = helpers::joined_stand_world(
        helpers::operator_datapack_config(),
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos {
            x: MOB_BLOCK.x,
            y: MOB_BLOCK.y - 1,
            z: MOB_BLOCK.z,
        },
        block_state_id("minecraft:dirt"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let leader = world.spawn_patrol_leader_for_test(Java26BuiltinDimension::Overworld, MOB_BLOCK);
    stage_one_blow_from_death(&mut world, leader);
    (sender, receiver, world, leader)
}

/// The `minecraft:ominous_bottle_amplifier` value one dropped item entity
/// carries, read off the persistent stack a save would write.
fn dropped_amplifier(world: &ServerWorld, entity_id: i32) -> Option<i32> {
    let stack = world.stored_item_stack_for_test(Java26BuiltinDimension::Overworld, entity_id)?;
    match crate::tests::saved_item_component(world, &stack, "minecraft:ominous_bottle_amplifier")? {
        NbtSummaryValue::Int(amplifier) => Some(amplifier),
        _ => None,
    }
}

/// `RaiderPredicate.CAPTAIN_WITHOUT_RAID` is
/// `{has_raid: false, is_captain: true}`, and `minecraft:entities/pillager`'s
/// only pool is gated on exactly it. `Raider.hasRaid()`
/// is `getCurrentRaid() != null ||
/// level.getRaidAt(blockPosition()) != null`: a patrol leader holds no current
/// raid, so the level's raid map is what answers the term, and this fixture's
/// map holds nothing.
///
/// The pool's `set_ominous_bottle_amplifier` is `uniform 0..=4`, so the
/// amplifier is a draw rather than a constant; what the assertion pins is that
/// the dropped bottle carries *an* amplifier the table wrote, which a refused
/// function could not leave behind.
#[test]
fn a_players_kill_drops_the_captains_ominous_bottle_with_the_amplifier_the_table_set() {
    let (sender, receiver, mut world, leader) = patrol_leader_kill_fixture();

    attack_by_player(&sender, &receiver, &mut world, leader);

    // Two drops, not one: the pool's bottle, and the captain's own preserved
    // head slot. `PatrollingMonster.finalizeSpawn` equips the ominous banner
    // and sets its drop chance to `2.0F`, and
    // `Mob.dropCustomDeathLoot` drops a preserved slot unconditionally --
    // `preserve` is `isPreserved`'s `> 1.0F`, the
    // gate is `killedByPlayer || preserve` and the draw is
    // `nextFloat() < dropChance`. Before the equipment
    // read projected through the level's publication this banner was refused
    // for the `minecraft:banner_pattern` holders it names, so the count of one
    // was the defect's, not vanilla's.
    let dropped = items(&world);
    assert_eq!(dropped.len(), 2, "{dropped:?}");
    let bottles: Vec<&StoredItem> = dropped
        .iter()
        .filter(|item| item.2 == "minecraft:ominous_bottle")
        .collect();
    assert_eq!(
        bottles.len(),
        1,
        "the captain pool draws one bottle: {dropped:?}"
    );
    assert_eq!(bottles[0].3, 1, "the pool's own `set_count` is 1");
    assert!(
        dropped
            .iter()
            .any(|item| item.2 == "minecraft:white_banner"),
        "the preserved head slot is the ominous banner, a named white banner: {dropped:?}"
    );
    // A drawn 0 is the item's own default, which saves no component.
    let amplifier = dropped_amplifier(&world, bottles[0].0).unwrap_or(0);
    assert!(
        (0..=4).contains(&amplifier),
        "the table's `uniform 0..=4` clamp produced {amplifier}"
    );
}

/// The contrasting arm of the same table: a pillager that is not a patrol
/// leader answers `is_captain` false, which the predicate's conjunction decides
/// without reading `has_raid` at all — the pool is empty and the map is not
/// consulted.
#[test]
fn a_plain_pillagers_kill_draws_no_ominous_bottle() {
    let (sender, receiver, mut world, pillager) =
        player_kill_fixture(VANILLA_ENTITY_TYPE_PILLAGER_ID, None);

    attack_by_player(&sender, &receiver, &mut world, pillager);

    assert!(items(&world).is_empty(), "{:?}", items(&world));
}

/// One `minecraft:entities/iron_golem` table replaced by a body whose only pool
/// drops one iron ingot with `set_components` on it, so the death path reaches
/// the function no built-in `minecraft:entities/*` table names.
fn iron_golem_kill_fixture(
    table: &serde_json::Value,
    looting_sword: bool,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks =
        datapack_config_with_loot_table("minecraft:entities/iron_golem", table).datapacks;
    let (sender, receiver, mut world) = helpers::joined_stand_world(
        config,
        Vec3d {
            x: 2.5,
            y: 64.0,
            z: 0.5,
        },
    );
    if looting_sword {
        let mut sword = plain_summary("minecraft:iron_sword", 1);
        enchant(&world, &mut sword, &[("minecraft:looting", 3)]);
        world.set_inventory_slot_for_test(ConnectionId(1), 0, sword);
    }
    let golem = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
    stage_one_blow_from_death(&mut world, golem);
    (sender, receiver, world, golem)
}

fn set_components_kill_fixture(
    components: serde_json::Value,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let table = serde_json::json!({
        "type": "minecraft:entity",
        "pools": [{
            "rolls": 1.0,
            "entries": [{
                "type": "minecraft:item",
                "name": "minecraft:iron_ingot",
                "functions": [{
                    "function": "minecraft:set_components",
                    "components": components,
                }],
            }],
        }],
    });
    iron_golem_kill_fixture(&table, false)
}

/// `SetComponentsFunction.run` on the real
/// death path: the components it writes into the drawn stack are on the item
/// entity in the persistent form a save writes, and that form decodes back to
/// the same summary — the round trip `/give <item>[components]` and `/item
/// modify` validate through.
///
/// The two `!`-prefixed entries are the patch's removal arm
/// read against the item prototype the way
/// `DataComponentPatch.applyPatch` does: an iron ingot declares
/// `minecraft:rarity` and `minecraft:repair_cost`, so removing them leaves the
/// `!` entry, while `minecraft:damage` has no default for this item and the
/// removal is the no-op vanilla writes nothing for.
#[test]
fn a_death_path_set_components_write_round_trips_through_the_persistent_projection() {
    let (sender, receiver, mut world, golem) = set_components_kill_fixture(serde_json::json!({
        "minecraft:repair_cost": 7,
        "minecraft:ominous_bottle_amplifier": 3,
        "!minecraft:rarity": {},
        "!minecraft:damage": {},
    }));

    attack_by_player(&sender, &receiver, &mut world, golem);

    let dropped = items(&world);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    let persistent = world
        .stored_item_stack_for_test(Java26BuiltinDimension::Overworld, dropped[0].0)
        .expect("the drop is a stored item entity");
    let component = |id| crate::tests::saved_item_component(&world, &persistent, id);
    for (id, expected) in [
        ("minecraft:repair_cost", NbtSummaryValue::Int(7)),
        (
            "minecraft:ominous_bottle_amplifier",
            NbtSummaryValue::Int(3),
        ),
    ] {
        assert_eq!(component(id), Some(expected), "{id}");
    }
    assert!(
        component("!minecraft:rarity").is_some(),
        "the removal of a component the item declares is kept"
    );
    assert!(
        component("!minecraft:damage").is_none() && component("minecraft:damage").is_none(),
        "a removal of a component the item does not declare writes nothing"
    );

    let resolver = world.item_stack_resolver();
    let saved = persistent.write(&resolver).unwrap().unwrap();
    let round_trip =
        bbb_server_types::item_stack::RuntimeItemStack::try_read(&saved, &resolver).unwrap();
    assert_eq!(round_trip, persistent);
}

/// The level of one enchantment on a dropped item entity, read off the
/// persistent stack a save writes.
///
/// `ItemEnchantments.CODEC` names each holder by *identifier*, so finding
/// `minecraft:sharpness` in the written compound at all is what the level's
/// publication bought: without it the projection has no name to write.
fn dropped_enchantment_level(
    world: &ServerWorld,
    entity_id: i32,
    enchantment: &str,
) -> Option<i32> {
    let stack = world.stored_item_stack_for_test(Java26BuiltinDimension::Overworld, entity_id)?;
    let NbtSummaryValue::Compound(entries) =
        crate::tests::saved_item_component(world, &stack, "minecraft:enchantments")?
    else {
        return None;
    };
    match entries
        .iter()
        .find(|entry| entry.name == enchantment)?
        .value
    {
        NbtSummaryValue::Int(level) => Some(level),
        NbtSummaryValue::Byte(level) => Some(i32::from(level)),
        _ => None,
    }
}

/// The client-visible half of the same drop: the watching session is sent the
/// `ItemEntity.DATA_ITEM` row with the component intact, and the stack it is
/// shown is the stack the inventory takes when the player walks onto it.
///
/// `ServerEntity.sendPairingData` publishes `ServerEntity.trackedDataValues`
/// which is `getNonDefaultValues`
/// (`SynchedEntityData.java:92-106`) -- a projection of the *persistent* stack
/// the entity holds (`ItemEntity.getItem`). Handling
/// that stack to a client means naming the enchantment holder, which only the
/// level's publication can do, and vanilla has no failure here at all:
/// `SynchedEntityData.DataValue.write` writes the row
/// unconditionally against the connection's own registry access.
///
/// The pickup half is `ItemEntity.playerTouch`
/// reached by the real walk loop, over the same publication, so the stack the
/// session was shown is the one `Inventory.add` receives.
#[test]
fn a_watching_session_is_sent_the_enchanted_drop_and_takes_it_off_the_ground() {
    let (sender, receiver, mut world, golem) = set_components_kill_fixture(serde_json::json!({
        "minecraft:enchantments": {"minecraft:sharpness": 3},
    }));

    let mut events = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::Attack(AttackEntity { entity_id: golem }),
    )
    .events;
    let dropped = items(&world);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    let (drop_id, drop_position, ..) = dropped[0].clone();

    // The pairing may land in the tick the drop did or in the next one.
    let mut sent = None;
    for _ in 0..4 {
        sent = sent.or_else(|| sent_item_row(&events, drop_id));
        if sent.is_some() {
            break;
        }
        events = world.tick(receiver.drain()).events;
    }
    let shown = sent.expect("the watching session is sent the drop's item row");

    let persistent = world
        .stored_item_stack_for_test(Java26BuiltinDimension::Overworld, drop_id)
        .expect("the drop is a stored item entity");
    let held = persistent.summary().clone();
    assert_eq!(
        shown, held,
        "the row the session is shown is the stack the server holds"
    );
    let mut expected = plain_summary("minecraft:iron_ingot", 1);
    enchant(&world, &mut expected, &[("minecraft:sharpness", 3)]);
    assert_eq!(
        shown, expected,
        "the row carries the component the table wrote, by published holder id"
    );

    // `Player.touch` over the drop: the session walks
    // onto it and the pickup loop reads the stack off the entity.
    stand_at(&sender, &receiver, &mut world, 1, drop_position);
    let mut picked = None;
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        if tick.events.iter().any(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::TakeItemEntity(_),
                    ..
                }
            )
        }) {
            picked = Some(tick);
            break;
        }
    }
    let picked = picked.expect("the player picks the drop up");
    assert!(items(&world).is_empty(), "the drop left the ground");
    assert_eq!(
        snapshot_player(&picked, 1)
            .inventory
            .slots
            .iter()
            .filter(|slot| **slot == shown)
            .count(),
        1
    );

    // The slot the inventory filled is published with the take
    // (`ServerPlayer.take`), which is the
    // snapshot the client's own menu holds.
    let settled = world.tick(receiver.drain());
    let inventory = &snapshot_player(&settled, 1).inventory.slots;
    assert_eq!(
        inventory.iter().filter(|slot| **slot == shown).count(),
        1,
        "the inventory took the stack it was shown, once: {inventory:?}"
    );
}

/// The `ItemEntity.DATA_ITEM` row one tick's packets carry for an entity, as the
/// session receives it.
fn sent_item_row(events: &[WorldEvent], entity_id: i32) -> Option<ItemStackSummary> {
    events.iter().find_map(|event| match event {
        WorldEvent::Send {
            packet: ClientboundPlayPacket::SetEntityData(data),
            ..
        } if data.id == entity_id => data.values.iter().find_map(|value| match &value.value {
            EntityDataValueKind::ItemStack(stack) => Some(stack.clone()),
            _ => None,
        }),
        _ => None,
    })
}

/// The acceptance of the drop boundary's publication: a mob a player killed
/// drops a stack carrying a component that names a registry holder, the item
/// entity lands with that component intact, and it survives a real save and
/// reload.
///
/// The component is `minecraft:enchantments`, whose `Enchantment.CODEC` is a
/// `RegistryFixedCodec` that writes the holder's identifier.
/// The world's
/// publication is `ServerWorld::item_stack_resolver()`, which
/// `level_and_drop_tables` already assembles for the loot draw beside
/// `BlockDropTables`; the drop downstream of that draw projects through the
/// same one, so nothing is lost between evaluating the table and placing the
/// entity.
#[test]
fn a_players_kill_drops_a_holder_resolved_component_through_a_real_save_and_reload() {
    let (sender, receiver, mut world, golem) = set_components_kill_fixture(serde_json::json!({
        "minecraft:enchantments": {"minecraft:sharpness": 3},
    }));

    attack_by_player(&sender, &receiver, &mut world, golem);

    let dropped = items(&world);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    assert_eq!(dropped[0].2, "minecraft:iron_ingot");
    assert_eq!(
        dropped_enchantment_level(&world, dropped[0].0, "minecraft:sharpness"),
        Some(3),
        "the item entity landed with the component the table wrote"
    );

    // The same entity region a restart reads, with the mob still inside its
    // death animation beside the drop.
    let request = world
        .save_request(1_786_000_000_999, 200, RegionCompression::Zlib)
        .expect("the save is produced");
    assert!(!saved_entities(&request).is_empty());
    let mut candidate = restore_candidate(world.game_time(), None, Vec::new());
    restore_entity_writes(&mut candidate, &request.entity_writes);
    let restored = spawn_pinned_restore(datapack_config(), &candidate);

    let reloaded = items(&restored);
    assert_eq!(reloaded.len(), 1, "{reloaded:?}");
    assert_eq!(reloaded[0].2, "minecraft:iron_ingot");
    assert_eq!(
        dropped_enchantment_level(&restored, reloaded[0].0, "minecraft:sharpness"),
        Some(3),
        "the reloaded item entity still names the enchantment"
    );
}

/// One `minecraft:entities/iron_golem` table replaced by a body whose only pool
/// drops one iron ingot through `minecraft:filtered`, gated on the death
/// context the pool reads: a player's kill, a direct blow, and a looting level
/// off the killer's main hand. No built-in `minecraft:entities/*` table names
/// the function, so the table is the only way to reach it.
///
/// The killer holds a looting III sword and the pool's
/// `random_chance_with_enchanted_bonus` is `0.0` unenchanted against a constant
/// `1.0` enchanted chance, so the pool runs exactly when the level is above
/// zero — `RandomSource.nextFloat()` is always below `1.0`.
///
/// `before` is an optional function the entry runs ahead of `filtered`, which
/// is how a case gives the drawn stack the components its `item_filter` reads.
fn filtered_kill_fixture(
    item_filter: serde_json::Value,
    before: Option<serde_json::Value>,
    on_pass: serde_json::Value,
    on_fail: serde_json::Value,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let mut functions: Vec<serde_json::Value> = before.into_iter().collect();
    functions.push(serde_json::json!({
        "function": "minecraft:filtered",
        "item_filter": item_filter,
        "on_pass": on_pass,
        "on_fail": on_fail,
    }));
    let table = serde_json::json!({
        "type": "minecraft:entity",
        "pools": [{
            "rolls": 1.0,
            "conditions": [
                {"condition": "minecraft:killed_by_player"},
                {
                    "condition": "minecraft:damage_source_properties",
                    "predicate": {"is_direct": true},
                },
                {
                    "condition": "minecraft:random_chance_with_enchanted_bonus",
                    "enchantment": "minecraft:looting",
                    "unenchanted_chance": 0.0,
                    "enchanted_chance": {
                        "type": "minecraft:linear",
                        "base": 1.0,
                        "per_level_above_first": 0.0,
                    },
                },
            ],
            "entries": [{
                "type": "minecraft:item",
                "name": "minecraft:iron_ingot",
                "functions": functions,
            }],
        }],
    });
    iron_golem_kill_fixture(&table, true)
}

/// `FilteredFunction.run` evaluates `item_filter`
/// against the drawn stack (including `minecraft:enchantments` holders resolved
/// through the level's publication) in a `killed_by_player` pool.
#[test]
fn a_death_path_filtered_function_answers_the_drawn_stacks_own_item_predicate() {
    let set_count =
        |count: i32| serde_json::json!({"function": "minecraft:set_count", "count": count});
    for (item_filter, before, on_pass, on_fail, expected) in [
        (
            serde_json::json!({"items": "minecraft:iron_ingot", "count": {"min": 1}}),
            None,
            set_count(5),
            set_count(3),
            vec![("minecraft:iron_ingot".to_string(), 5)],
        ),
        (
            serde_json::json!({"items": "minecraft:diamond"}),
            None,
            set_count(5),
            set_count(3),
            vec![("minecraft:iron_ingot".to_string(), 3)],
        ),
        (
            serde_json::json!({
                "predicates": {
                    "minecraft:enchantments": [{
                        "enchantments": "minecraft:sharpness",
                        "levels": {"min": 3},
                    }],
                },
            }),
            Some(serde_json::json!({
                "function": "minecraft:set_components",
                "components": {"minecraft:enchantments": {"minecraft:sharpness": 3}},
            })),
            set_count(5),
            set_count(3),
            vec![("minecraft:iron_ingot".to_string(), 5)],
        ),
    ] {
        let (sender, receiver, mut world, golem) = filtered_kill_fixture(
            item_filter.clone(),
            before.clone(),
            on_pass.clone(),
            on_fail.clone(),
        );

        attack_by_player(&sender, &receiver, &mut world, golem);

        assert_eq!(
            dropped_stacks(&world),
            expected,
            "{item_filter} with {on_pass} / {on_fail}"
        );
    }
}

/// The `minecraft:enchant_with_levels` node with the `levels` and `options` one
/// case declares, as a loot function document.
fn enchant_with_levels_node(levels: i32, options: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "function": "minecraft:enchant_with_levels",
        "levels": levels,
        "options": options,
    })
}

/// A player kill of an iron golem whose `minecraft:entities/iron_golem` table
/// is one `killed_by_player` pool of one `item` entry carrying the functions
/// under test.
///
/// A datapack table is the only shape that reaches this node: all 108 built-in
/// `minecraft:entities/*` files are free of it (the built-in writers are
/// `chests/*`, `gameplay/fishing/treasure` and the `villager_trade` entries the
/// merchant surface reads), so an entity table naming it is one a datapack
/// replaced. The conditions and the looting III sword are the ones the rest of
/// this file's kills use, so the draw stays the real death path's.
fn enchant_with_levels_kill_fixture(
    item: &str,
    functions: serde_json::Value,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let table = serde_json::json!({
        "type": "minecraft:entity",
        "pools": [{
            "rolls": 1.0,
            "conditions": [
                {"condition": "minecraft:killed_by_player"},
                {
                    "condition": "minecraft:damage_source_properties",
                    "predicate": {"is_direct": true},
                },
            ],
            "entries": [{
                "type": "minecraft:item",
                "name": item,
                "functions": functions,
            }],
        }],
    });
    iron_golem_kill_fixture(&table, true)
}

/// The `minecraft:enchantment` registry the world's generation carries, as
/// `(id, document)` in registry order.
fn enchantment_documents(world: &ServerWorld) -> Vec<(String, serde_json::Value)> {
    world
        .datapacks()
        .expect("the fixture world has a datapack generation")
        .registries
        .registry("minecraft:enchantment")
        .expect("the generation carries the enchantment registry")
        .elements
        .iter()
        .map(|element| {
            (
                element.id.clone(),
                serde_json::from_slice::<serde_json::Value>(&element.source_json).unwrap(),
            )
        })
        .collect()
}

/// One enchantment document out of [`enchantment_documents`].
fn enchantment_document<'a>(
    documents: &'a [(String, serde_json::Value)],
    id: &str,
) -> &'a serde_json::Value {
    &documents
        .iter()
        .find(|(name, _)| name == id)
        .unwrap_or_else(|| panic!("the generation carries {id}"))
        .1
}

/// The members of one `minecraft:enchantment` tag, as the generation's own tag
/// publication lists them -- the same publication the level installs into its
/// chunk store for the selection to read.
fn enchantment_tag_members(world: &ServerWorld, tag: &str) -> Vec<String> {
    world
        .datapacks()
        .expect("the fixture world has a datapack generation")
        .tags
        .registry("minecraft:enchantment")
        .and_then(|tags| tags.tag(tag))
        .unwrap_or_else(|| panic!("the generation declares {tag}"))
        .to_vec()
}

/// `HolderSet.contains` (`HolderSet.java`) for one holder-set field of an
/// enchantment or item document, over the generation's own tags: a `#tag` names
/// that tag's members, a plain id is itself, and an array is the union.
fn holder_set_contains(
    world: &ServerWorld,
    registry: &str,
    set: &serde_json::Value,
    id: &str,
) -> bool {
    match set {
        serde_json::Value::String(reference) => match reference.strip_prefix('#') {
            Some(tag) => world
                .datapacks()
                .expect("the fixture world has a datapack generation")
                .tags
                .registry(registry)
                .and_then(|tags| tags.tag(tag))
                .is_some_and(|members| members.iter().any(|member| member == id)),
            None => reference == id,
        },
        serde_json::Value::Array(values) => values
            .iter()
            .any(|value| holder_set_contains(world, registry, value, id)),
        _ => false,
    }
}

/// The declared `minecraft:enchantable` value of a builtin item, which is the
/// bound `EnchantmentHelper.selectEnchantment` takes its two draws from
fn item_enchantability(item: &str) -> i32 {
    let item = bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
        .unwrap_or_else(|| panic!("{item} is a builtin item"));
    let catalog = bbb_protocol::vanilla_item_prototype_component_catalog()
        .expect("the embedded prototype catalog builds");
    let value = catalog
        .component_codec_values(item.id)
        .and_then(|values| {
            values.get(
                &bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID,
            )
        })
        .unwrap_or_else(|| {
            panic!(
                "minecraft:{} declares no minecraft:enchantable",
                item.resource_id
            )
        });
    i32::try_from(
        value
            .get("value")
            .and_then(serde_json::Value::as_i64)
            .expect("the component carries an int value"),
    )
    .expect("the value fits i32")
}

/// One linear cost field of an enchantment document at `level`
/// (`LevelBasedValue.Linear.calculate`).
fn linear_cost(document: &serde_json::Value, field: &str, level: i32) -> i32 {
    let field = document
        .get(field)
        .unwrap_or_else(|| panic!("the document declares {field}"));
    let base = i32::try_from(
        field
            .get("base")
            .and_then(serde_json::Value::as_i64)
            .expect("base"),
    )
    .expect("base fits i32");
    let per_level = i32::try_from(
        field
            .get("per_level_above_first")
            .and_then(serde_json::Value::as_i64)
            .expect("per_level_above_first"),
    )
    .expect("per_level_above_first fits i32");
    base + per_level * (level - 1)
}

/// The levels one enchantment can be selected at, over every cost the declared
/// `levels` value and the item's own enchantability can produce.
///
/// `EnchantmentHelper.selectEnchantment` spends its two `nextInt` draws and its
/// span on the cost before it asks for candidates (`:559-561`), and
/// `getAvailableEnchantmentResults` adds the highest level in `1..=max_level`
/// whose `min_cost`/`max_cost` band holds that cost (`:597-611`). The candidate
/// list is built once, so the halved cost the extra-pick loop advances
/// (`:566-577`) never reselects a level.
///
/// This is the oracle the assertions hold the drawn selection to: it is a
/// bound over the cost arithmetic, not a second selection -- it answers no
/// weight, no compatibility and no draw.
fn reachable_levels(document: &serde_json::Value, enchantability: i32, levels: i32) -> Vec<i32> {
    let max_level = i32::try_from(
        document
            .get("max_level")
            .and_then(serde_json::Value::as_i64)
            .expect("max_level"),
    )
    .expect("max_level fits i32");
    let bound = enchantability / 4 + 1;
    let mut reachable = std::collections::BTreeSet::new();
    for first in 0..bound {
        for second in 0..bound {
            // `nextFloat()` is below `1.0`, so the span stays inside these ends.
            for span in [-0.15_f32, 0.15_f32] {
                let raw = levels + 1 + first + second;
                let cost =
                    ((f64::from(raw as f32 + raw as f32 * span) + 0.5).floor() as i32).max(1);
                for level in (1..=max_level).rev() {
                    if cost >= linear_cost(document, "min_cost", level)
                        && cost <= linear_cost(document, "max_cost", level)
                    {
                        reachable.insert(level);
                        break;
                    }
                }
            }
        }
    }
    reachable.into_iter().collect()
}

/// One enchantment component of a dropped item entity, as `(id, level)` sorted.
///
/// `ItemEnchantments.CODEC` writes each holder by identifier and the level as an
/// int, so the ids here are the ones the
/// selection named rather than the holder indices a summary carries.
fn dropped_enchantment_component(
    world: &ServerWorld,
    entity_id: i32,
    component_id: &str,
) -> Vec<(String, i32)> {
    let Some(stack) =
        world.stored_item_stack_for_test(Java26BuiltinDimension::Overworld, entity_id)
    else {
        return Vec::new();
    };
    let Some(NbtSummaryValue::Compound(entries)) =
        crate::tests::saved_item_component(world, &stack, component_id)
    else {
        return Vec::new();
    };
    let mut written: Vec<(String, i32)> = entries
        .iter()
        .filter_map(|entry| match &entry.value {
            NbtSummaryValue::Int(level) => Some((entry.name.to_jdk_string(), *level)),
            NbtSummaryValue::Byte(level) => Some((entry.name.to_jdk_string(), i32::from(*level))),
            _ => None,
        })
        .collect();
    written.sort();
    written
}

/// `EnchantWithLevelsFunction.run` on
/// the real death path, over a named `options` entry.
///
/// The cost the node samples is not observable, so the assertion is the one the
/// draw has to satisfy: `minecraft:sharpness` is the only candidate, and its
/// level is the highest whose `min_cost`/`max_cost` band holds a cost the
/// declared `levels: 30` value and the sword's own enchantability can produce.
/// Those levels are a proper
/// subset of `1..=max_level`, so the arm can tell a level the cost arithmetic
/// admits from one it does not.
#[test]
fn a_death_path_enchant_with_levels_selects_its_named_option_over_the_level_cost() {
    let (sender, receiver, mut world, golem) = enchant_with_levels_kill_fixture(
        "minecraft:iron_sword",
        serde_json::json!([enchant_with_levels_node(
            30,
            serde_json::json!("minecraft:sharpness")
        )]),
    );

    attack_by_player(&sender, &receiver, &mut world, golem);

    let dropped = items(&world);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    assert_eq!(dropped[0].2, "minecraft:iron_sword");
    assert_eq!(dropped[0].3, 1);

    let documents = enchantment_documents(&world);
    let document = enchantment_document(&documents, "minecraft:sharpness");
    let reachable = reachable_levels(document, item_enchantability("minecraft:iron_sword"), 30);
    let max_level = i32::try_from(
        document
            .get("max_level")
            .and_then(serde_json::Value::as_i64)
            .expect("max_level"),
    )
    .unwrap();
    assert!(
        !reachable.is_empty() && reachable.len() < usize::try_from(max_level).unwrap(),
        "the cost band leaves levels out, so the level assertion has teeth: {reachable:?}"
    );

    let selected = dropped_enchantment_component(&world, dropped[0].0, "minecraft:enchantments");
    assert_eq!(selected.len(), 1, "{selected:?}");
    assert_eq!(selected[0].0, "minecraft:sharpness");
    assert!(
        reachable.contains(&selected[0].1),
        "level {} is one the declared 30 levels can select: {reachable:?}",
        selected[0].1
    );
}

/// The same node over `options` naming a tag: `#minecraft:on_random_loot` is a
/// `minecraft:enchantment` tag, so the members come from the generation's own
/// tag publication -- the one the level installs into its chunk store
/// (`ChunkStore::registry_tags`) and `spawner_enchantment_holders` reads.
///
/// The draw is random, so the assertions are the terms vanilla selects by:
/// every selected member of the tag, at a
/// level the declared cost can reach, on an item the enchantment
/// supports and counts as primary, pairwise
/// compatible.
#[test]
fn a_death_path_enchant_with_levels_selects_from_a_tag_over_the_generations_own_members() {
    let (sender, receiver, mut world, golem) = enchant_with_levels_kill_fixture(
        "minecraft:iron_sword",
        serde_json::json!([enchant_with_levels_node(
            30,
            serde_json::json!("#minecraft:on_random_loot")
        )]),
    );

    attack_by_player(&sender, &receiver, &mut world, golem);

    let dropped = items(&world);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    assert_eq!(dropped[0].2, "minecraft:iron_sword");

    let members = enchantment_tag_members(&world, "minecraft:on_random_loot");
    let documents = enchantment_documents(&world);
    let enchantability = item_enchantability("minecraft:iron_sword");
    let selected = dropped_enchantment_component(&world, dropped[0].0, "minecraft:enchantments");
    assert!(
        !selected.is_empty(),
        "the node selected over the tag rather than refusing it"
    );
    for (id, level) in &selected {
        assert!(
            members.contains(id),
            "{id} is a #minecraft:on_random_loot member"
        );
        let document = enchantment_document(&documents, id);
        let reachable = reachable_levels(document, enchantability, 30);
        assert!(
            reachable.contains(level),
            "{id} at level {level} is one the declared 30 levels can select: {reachable:?}"
        );
        assert!(
            holder_set_contains(
                &world,
                "minecraft:item",
                document.get("supported_items").expect("supported_items"),
                "minecraft:iron_sword"
            ),
            "{id} supports the item it was written onto"
        );
        if let Some(primary) = document.get("primary_items") {
            assert!(
                holder_set_contains(&world, "minecraft:item", primary, "minecraft:iron_sword"),
                "{id} counts the item it was written onto as primary"
            );
        }
    }
    for (first, _) in &selected {
        for (second, _) in &selected {
            if first == second {
                continue;
            }
            let exclude = |a: &str, b: &str| {
                enchantment_document(&documents, a)
                    .get("exclusive_set")
                    .is_some_and(|set| holder_set_contains(&world, "minecraft:enchantment", set, b))
            };
            assert!(
                !exclude(first, second) && !exclude(second, first),
                "{first} and {second} are not in each other's exclusive_set"
            );
        }
    }
}

/// The book conversion of the same node: `EnchantmentHelper.enchantItem`
/// replaces a `minecraft:book` with a fresh `minecraft:enchanted_book` before
/// it writes anything, so the count the
/// table's `set_count` set is dropped and the selection lands on
/// `minecraft:stored_enchantments` rather than `minecraft:enchantments`
#[test]
fn a_death_path_enchant_with_levels_turns_a_book_into_a_stored_enchantment_book() {
    for random_enchant in [false, true] {
        let (sender, receiver, mut world, golem) = enchant_with_levels_kill_fixture(
            "minecraft:book",
            serde_json::json!([
                {"function": "minecraft:set_count", "count": 3},
                if random_enchant {
                    serde_json::json!({"function":"minecraft:enchant_randomly", "options":"#minecraft:on_random_loot"})
                } else {
                    enchant_with_levels_node(30, serde_json::json!("#minecraft:on_random_loot"))
                },
            ]),
        );

        attack_by_player(&sender, &receiver, &mut world, golem);

        let dropped = items(&world);
        assert_eq!(dropped.len(), 1, "{dropped:?}");
        assert_eq!(dropped[0].2, "minecraft:enchanted_book");
        assert_eq!(dropped[0].3, 1, "the conversion replaced the counted book");

        let members = enchantment_tag_members(&world, "minecraft:on_random_loot");
        let documents = enchantment_documents(&world);
        let enchantability = item_enchantability("minecraft:book");
        let stored =
            dropped_enchantment_component(&world, dropped[0].0, "minecraft:stored_enchantments");
        assert!(!stored.is_empty(), "the book stored what it selected");
        assert!(
            dropped_enchantment_component(&world, dropped[0].0, "minecraft:enchantments")
                .is_empty(),
            "an enchanted book stores rather than enchants"
        );
        for (id, level) in &stored {
            assert!(
                members.contains(id),
                "{id} is a #minecraft:on_random_loot member"
            );
            if random_enchant {
                let max = enchantment_document(&documents, id)
                    .get("max_level")
                    .unwrap()
                    .as_i64()
                    .unwrap() as i32;
                assert!((1..=max).contains(level));
                continue;
            }
            let reachable =
                reachable_levels(enchantment_document(&documents, id), enchantability, 30);
            assert!(
                reachable.contains(level),
                "{id} at level {level} is one the declared 30 levels can select: {reachable:?}"
            );
        }
    }
}

/// The empty-selection arm: `minecraft:stick` declares no
/// `minecraft:enchantable`, so `selectEnchantment` answers nothing before it
/// draws and the item is handed back as it
/// was -- a drawn stack, not a refused table.
#[test]
fn a_death_path_enchant_with_levels_leaves_an_item_it_cannot_enchant_alone() {
    let (sender, receiver, mut world, golem) = enchant_with_levels_kill_fixture(
        "minecraft:stick",
        serde_json::json!([enchant_with_levels_node(
            30,
            serde_json::json!("#minecraft:on_random_loot")
        )]),
    );

    attack_by_player(&sender, &receiver, &mut world, golem);

    let dropped = items(&world);
    assert_eq!(dropped.len(), 1, "{dropped:?}");
    assert_eq!(dropped[0].2, "minecraft:stick");
    assert_eq!(dropped[0].3, 1);
    assert!(
        dropped_enchantment_component(&world, dropped[0].0, "minecraft:enchantments").is_empty()
    );
    assert!(
        dropped_enchantment_component(&world, dropped[0].0, "minecraft:stored_enchantments")
            .is_empty()
    );
}

mod mob_experience;
