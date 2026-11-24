//! The world-tick flush of each session's advancement store and the datapack
//! install rebuild that reloads those stores over their written-back progress.

use super::*;
use crate::connection::ClientboundPlayPacket;
use bbb_server_persistence::{Java26PlayerAdvancements, JAVA_26_1_DATA_VERSION};

mod hurt;
mod item_durability;
mod killed_criteria;
mod player_predicate;
mod raw_source;
mod sheared_equipment;

fn world_with_datapacks() -> ServerWorld {
    ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup_generation()),
        ..ServerWorldConfig::default()
    })
    .unwrap()
}

fn progress_document(id: &str, criterion: &str) -> Java26PlayerAdvancements {
    Java26PlayerAdvancements::parse_bytes(
        &serde_json::to_vec(&serde_json::json!({
            id: {
                "criteria": { criterion: "2026-08-31 02:30:45 +0000" },
                "done": true,
            },
            "DataVersion": JAVA_26_1_DATA_VERSION,
        }))
        .unwrap(),
    )
    .expect("the fixture document is well formed")
}

fn update_advancements(
    tick: &TickResult,
    connection_id: u64,
) -> Vec<&bbb_protocol::packets::UpdateAdvancements> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: id,
                packet: ClientboundPlayPacket::UpdateAdvancements(update),
            } if id.0 == connection_id => Some(update),
            _ => None,
        })
        .collect()
}

#[test]
fn a_restored_store_flushes_update_advancements_on_the_join_tick() {
    let (sender, receiver) = mailbox();
    let mut world = world_with_datapacks();
    world.seed_restored_player_advancements(
        profile("alice", 1).uuid,
        progress_document("minecraft:story/root", "crafting_table"),
    );
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);

    let updates = update_advancements(&joined, 1);
    assert_eq!(updates.len(), 1, "{joined:?}");
    let update = updates[0];
    assert!(update.reset);
    assert!(update.show_advancements);
    assert!(
        update
            .added
            .iter()
            .any(|added| added.id == "minecraft:story/root"),
        "{update:?}"
    );
    assert!(
        update.progress.iter().any(|progress| {
            progress.id == "minecraft:story/root"
                && progress.criteria.iter().any(|criterion| {
                    criterion.name == "crafting_table" && criterion.obtained_epoch_millis.is_some()
                })
        }),
        "{update:?}"
    );
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:story/root"));

    assert!(
        update_advancements(&handshake, 1).is_empty(),
        "{handshake:?}"
    );
    let idle = world.tick(vec![]);
    assert!(update_advancements(&idle, 1).is_empty(), "{idle:?}");
}

#[test]
fn a_world_without_datapacks_sends_no_advancement_update() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let (joined, handshake) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(update_advancements(&joined, 1).is_empty(), "{joined:?}");
    assert!(
        update_advancements(&handshake, 1).is_empty(),
        "{handshake:?}"
    );
}

/// The decorated pot icon the projection now owns reaches the session instead.
#[test]
fn the_decorated_pot_icon_reaches_the_session() {
    let (sender, receiver) = mailbox();
    let mut world = world_with_datapacks();
    world.seed_restored_player_advancements(
        profile("alice", 1).uuid,
        progress_document(
            "minecraft:adventure/craft_decorated_pot_using_only_sherds",
            "pot_crafted_using_only_sherds",
        ),
    );
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);

    assert_eq!(update_advancements(&joined, 1).len(), 1, "{joined:?}");
}

#[test]
fn datapack_install_reloads_every_store_over_its_written_back_progress() {
    let (sender, receiver) = mailbox();
    let startup = startup_generation();
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::clone(&startup)),
        ..ServerWorldConfig::default()
    })
    .unwrap();
    world.seed_restored_player_advancements(
        profile("alice", 1).uuid,
        progress_document("minecraft:story/root", "crafting_table"),
    );
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let before = world.player_advancements_to_data(ConnectionId(1));
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:story/root"));

    enqueue(
        &sender,
        WorldCommand::Datapacks {
            startup: ReloadedDatapacks::Built(Arc::clone(&startup)),
        },
    )
    .unwrap();
    let reloaded = world.tick(receiver.drain());
    assert_eq!(world.player_advancements_to_data(ConnectionId(1)), before);
    assert!(world.player_advancement_is_done(ConnectionId(1), "minecraft:story/root"));

    let updates = update_advancements(&reloaded, 1);
    assert_eq!(updates.len(), 1, "{reloaded:?}");
    assert!(updates[0].reset);
    assert!(
        updates[0]
            .added
            .iter()
            .any(|added| added.id == "minecraft:story/root"),
        "{:?}",
        updates[0]
    );
}

/// `recipes/decorations/crafting_table.json`'s `unlock_right_away` is the whole
/// of the built-in pack's `minecraft:tick` surface, and its requirement grid
/// makes that one criterion enough: every session that plays one tick has the
/// crafting table in its book.
#[test]
fn the_tick_criterion_unlocks_the_crafting_table_on_the_join_tick() {
    const CRAFTING_TABLE: &str = "minecraft:recipes/decorations/crafting_table";
    let (sender, receiver) = mailbox();
    let mut world = world_with_datapacks();
    let (joined, _) = join_loaded_ticks(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(
        world.player_advancement_is_done(ConnectionId(1), CRAFTING_TABLE),
        "{joined:?}"
    );
    assert!(world.player_knows_recipe_for_test(ConnectionId(1), "minecraft:crafting_table"));
    assert!(update_advancements(&joined, 1).iter().any(|update| update
        .progress
        .iter()
        .any(|progress| progress.id == CRAFTING_TABLE)));
}

/// The criteria one written `players/advancements/<uuid>.json` row names, which
/// is the obtained ones alone (`AdvancementProgress.java:29-35`).
fn written_criteria(
    data: &Java26PlayerAdvancements,
    id: &str,
) -> Option<Vec<bbb_protocol::JavaString>> {
    let progress = data.progress.get(id)?;
    Some(
        progress
            .criteria()
            .iter()
            .filter(|(_, criterion)| criterion.is_done())
            .map(|(name, _)| name.clone())
            .collect(),
    )
}

/// The gate's advancement leg over the built-in pack: `/recipe give <p> *`
/// completes every recipe advancement through its `minecraft:recipe_unlocked`
/// criterion, and `Action.GRANT` then skips the rows it already finished
/// (`AdvancementCommands.java:370-383`).
///
/// A session that never holds stone can only complete
/// `minecraft:recipes/building_blocks/stone_bricks` through that unlock: its
/// other criterion wants stone in the inventory, and the changed stack a fresh
/// session publishes is empty.
#[test]
fn the_recipe_unlock_is_what_completes_a_recipe_advancement_a_session_holds_nothing_for() {
    const STONE_BRICKS: &str = "minecraft:recipes/building_blocks/stone_bricks";
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(startup_generation());
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);

    // Only the tick criterion has fired: an empty inventory completes no
    // `minecraft:inventory_changed` criterion.
    let joined = world.player_advancements_to_data(ConnectionId(1));
    assert_eq!(
        joined.progress.keys().collect::<Vec<_>>(),
        ["minecraft:recipes/decorations/crafting_table"],
        "{joined:?}"
    );
    assert!(!world.player_advancement_is_done(ConnectionId(1), STONE_BRICKS));

    chat_command(&sender, &receiver, &mut world, 1, "recipe give alice *");
    assert_eq!(
        written_criteria(
            &world.player_advancements_to_data(ConnectionId(1)),
            STONE_BRICKS
        ),
        Some(vec!["has_the_recipe".into()])
    );

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "advancement grant alice everything",
    );
    let granted = world.player_advancements_to_data(ConnectionId(1));
    for (id, criteria) in [
        ("minecraft:story/root", vec!["crafting_table"]),
        (
            "minecraft:adventure/root",
            vec!["killed_by_something", "killed_something"],
        ),
        (
            "minecraft:recipes/decorations/crafting_table",
            vec!["unlock_right_away"],
        ),
        (STONE_BRICKS, vec!["has_the_recipe"]),
    ] {
        assert_eq!(
            written_criteria(&granted, id),
            Some(criteria.iter().map(|name| (*name).into()).collect()),
            "{id}"
        );
        assert!(
            world.player_advancement_is_done(ConnectionId(1), id),
            "{id}"
        );
    }
}

/// The advancement a player's kill of a stored mob completes on its own: its
/// criteria are one `minecraft:player_killed_entity` per hostile type and its
/// requirements take any one of them.
const KILL_A_MOB: &str = "minecraft:adventure/kill_a_mob";

/// `Stats.MOB_KILLS` (`ServerPlayer.java:950`), the counter the same
/// `awardKillScore` files a non-`Player` victim under.
fn mob_kills(world: &ServerWorld) -> i32 {
    world.player_stat_for_test(
        ConnectionId(1),
        bbb_server_types::stats::custom_stat("minecraft:mob_kills"),
    )
}

/// `Stats.ENTITY_KILLED` for zombies (`Player.java:1487`), which
/// `Player.killedEntity` files against the blow's own source entity.
fn zombies_killed(world: &ServerWorld) -> i32 {
    world.player_stat_for_test(
        ConnectionId(1),
        bbb_server_types::stats::StatKey::new(
            bbb_protocol::stat_types::VANILLA_STAT_TYPE_ENTITY_KILLED_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        ),
    )
}

/// An operator session over the built-in advancement generation, which is what
/// makes `minecraft:adventure/kill_a_mob` and `minecraft:husbandry/tame_an_animal`
/// real rows rather than fixtures.
fn built_in_advancement_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(startup_generation());
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    (sender, receiver, world)
}

fn spawn_zombie(world: &mut ServerWorld, x: f64) -> uuid::Uuid {
    world
        .spawn_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d { x, y: 64.0, z: 0.0 },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(
                    bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                )
                .expect("a zombie is a living entity"),
            ),
        )
        .1
}

/// `ServerPlayer.awardKillScore`'s own criterion (`ServerPlayer.java:955`) on
/// the real kill path: the stored victim's death credits the session that
/// dealt it, and the advancement that kill completes is granted.
#[test]
fn a_players_kill_of_a_stored_mob_grants_the_advancement_that_kill_completes() {
    let (sender, receiver) = mailbox();
    let table = serde_json::json!({"type":"minecraft:entity","pools":[
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"slots":{
                "weapon.*":{"items":"minecraft:diamond"},
                "armor.head":{"items":"minecraft:carved_pumpkin"},
                "armor.chest":{"count":0}
            }}}],"entries":[{"type":"minecraft:item","name":"minecraft:emerald"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"slots":{
                "weapon.*":{"items":"minecraft:diamond"},
                "armor.head":{"items":"minecraft:diamond"}
            }}}],"entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"slots":{"container.*":{"count":0}}}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:iron_ingot"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"attacker",
            "predicate":{"slots":{"inventory.0":{"items":"minecraft:diamond",
                "components":{"minecraft:custom_data":{"marked":17}}}}}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:lapis_lazuli"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"attacker",
            "predicate":{"slots":{"inventory.0":{"items":"minecraft:gold_ingot"}}}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:gold_ingot"}]}
    ]});
    let config = datapack_config_with_loot_table("minecraft:entities/zombie", &table);
    let mut startup = (**config.datapacks.as_ref().unwrap()).clone();
    for (id, predicate) in [
        (
            "test:moving_victim",
            serde_json::json!({"movement":{"speed":{"max":1000}}}),
        ),
        (
            "test:wrong_movement",
            serde_json::json!({"movement":{"speed":{"min":1000}}}),
        ),
        ("test:empty_slots", serde_json::json!({"slots":{}})),
    ] {
        startup.advancements.advancements.push(
            bbb_server_datapack::advancement::AdvancementElement {
                id: id.into(),
                source_pack: "test".into(),
                advancement: crate::tests::parse_advancement(
                    &serde_json::json!({"criteria":{"kill":{
                        "trigger":"minecraft:player_killed_entity",
                        "conditions":{"entity":[{
                            "condition":"minecraft:entity_properties", "entity":"this",
                            "predicate":predicate
                        }],"player":{"slots":{"inventory.0":{"items":"minecraft:diamond",
                            "components":{"minecraft:custom_data":{"marked":17}}}}}}
                    }}}),
                )
                .unwrap(),
            },
        );
    }
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(std::sync::Arc::new(startup));
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    let zombie = spawn_zombie(&mut world, 2.0);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity alice inventory.0 with diamond[custom_data={marked:17b}]",
    );
    for (slot, item) in [
        ("weapon.offhand", "diamond"),
        ("armor.head", "carved_pumpkin"),
    ] {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity {zombie} {slot} with {item}"),
        );
    }
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("data merge entity {zombie} {{drop_chances:{{offhand:0f,head:0f}}}}"),
    );
    assert!(!world.player_advancement_is_done(ConnectionId(1), KILL_A_MOB));

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {zombie} 40 minecraft:generic by alice"),
    );

    assert!(
        world.player_advancement_is_done(ConnectionId(1), KILL_A_MOB),
        "`minecraft:player_killed_entity` has a producer for a stored victim"
    );
    let written = world.player_advancements_to_data(ConnectionId(1));
    assert_eq!(
        written_criteria(&written, KILL_A_MOB),
        Some(vec!["minecraft:zombie".into()]),
        "one criterion of the row, the one naming the victim's type"
    );
    assert_eq!(
        written_criteria(&written, "minecraft:adventure/root"),
        Some(vec!["killed_something".into()]),
        "the root takes the killer's side of any kill"
    );
    // `:949-951` and `Player.java:1486-1489`: both counters of the same death,
    // each on the player its own hook names.
    assert_eq!(mob_kills(&world), 1);
    assert_eq!(zombies_killed(&world), 1);
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:moving_victim"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:wrong_movement"));
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:empty_slots"));
    let drops: Vec<_> = world
        .stored_items_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .map(|item| (item.2, item.3))
        .collect();
    assert_eq!(
        drops,
        vec![
            ("minecraft:emerald".to_string(), 1),
            ("minecraft:lapis_lazuli".to_string(), 1)
        ]
    );
}

/// The two hooks `LivingEntity.die` reaches are not one: the credit comes off
/// the `lastHurtByPlayer` field (`LivingEntity.java:1947-1948`), while the
/// statistic needs the blow's own source entity (`:1446`). A player who
/// softened a mob and let something sourceless finish it has the first and not
/// the second.
#[test]
fn a_remembered_killer_takes_the_criterion_a_sourceless_blow_files_no_statistic_for() {
    let (sender, receiver, mut world) = built_in_advancement_world();
    let zombie = spawn_zombie(&mut world, 2.0);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {zombie} 1 minecraft:generic by alice"),
    );
    assert!(
        !world.player_advancement_is_done(ConnectionId(1), KILL_A_MOB),
        "a hit the mob survived credits nothing"
    );
    for _ in 0..20 {
        world.tick(receiver.drain());
    }

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {zombie} 40 minecraft:generic"),
    );

    assert!(
        world.player_advancement_is_done(ConnectionId(1), KILL_A_MOB),
        "the remembered player is still the kill credit"
    );
    assert_eq!(mob_kills(&world), 1);
    assert_eq!(
        zombies_killed(&world),
        0,
        "a blow with no source entity has no `Player.killedEntity` receiver"
    );
}

/// `getKillCredit`'s second arm is `lastHurtByMob` (`LivingEntity.java:1950`),
/// which is a `LivingEntity` and never a criterion's receiver: a mob's kill of
/// a mob reaches no session at all, and nothing is refused on one either.
#[test]
fn a_mob_killed_by_a_mob_credits_no_session_and_refuses_nothing() {
    let (sender, receiver, mut world) = built_in_advancement_world();
    let victim = spawn_zombie(&mut world, 2.0);
    let killer = spawn_zombie(&mut world, 4.0);

    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        &format!("damage {victim} 40 minecraft:generic by {killer}"),
    );

    assert!(!world.player_advancement_is_done(ConnectionId(1), KILL_A_MOB));
    assert_eq!(mob_kills(&world), 0);
    assert_eq!(zombies_killed(&world), 0);
}

/// The advancement one taming completes on its own: a single
/// `minecraft:tame_animal` criterion with no condition at all.
const TAME_AN_ANIMAL: &str = "minecraft:husbandry/tame_an_animal";

fn spawn_tamable_beside_alice(world: &mut ServerWorld, entity_type: i32, variant: &str) -> i32 {
    let position = world.publication().players[0].position;
    let dimension = Java26BuiltinDimension::Overworld;
    let id = world
        .spawn_stored_entity_for_test(
            dimension,
            entity_type,
            Vec3d {
                x: position.x + 1.5,
                ..position
            },
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(entity_type).unwrap(),
            ),
        )
        .0;
    let bbb_protocol::NbtSummaryValue::Compound(mut entries) = world
        .stored_entity_command_data_for_test(dimension, id)
        .unwrap()
    else {
        panic!("an entity record is a compound")
    };
    entries.retain(|entry| {
        entry.name != "variant" && entry.name != "NoAI" && entry.name != "NoGravity"
    });
    for (name, value) in [
        (
            "variant",
            bbb_protocol::NbtSummaryValue::String(variant.into()),
        ),
        ("NoAI", bbb_protocol::NbtSummaryValue::Byte(1)),
        ("NoGravity", bbb_protocol::NbtSummaryValue::Byte(1)),
    ] {
        entries.push(bbb_protocol::NbtSummaryEntry {
            name: name.into(),
            value,
        });
    }
    world.load_stored_entity_record_for_test(dimension, id, &entries);
    id
}

fn feed_tamable(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    animal: i32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::Interact(InteractEntity {
            entity_id: animal,
            hand: InteractionHand::MainHand,
            location: Vec3d::default(),
            using_secondary_action: false,
        }),
    )
}

#[test]
fn a_real_taming_grants_the_advancement_one_tamed_animal_completes() {
    let (sender, receiver, mut world) = built_in_advancement_world();
    assert!(!world.player_advancement_is_done(ConnectionId(1), TAME_AN_ANIMAL));
    for (entity_type, food, advancement, variants) in [
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID,
            "minecraft:bone",
            "minecraft:husbandry/whole_pack",
            &[
                "ashen", "black", "chestnut", "pale", "rusty", "snowy", "spotted", "striped",
                "woods",
            ][..],
        ),
        (
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAT_ID,
            "minecraft:cod",
            "minecraft:husbandry/complete_catalogue",
            &[
                "all_black",
                "black",
                "british_shorthair",
                "calico",
                "jellie",
                "persian",
                "ragdoll",
                "red",
                "siamese",
                "tabby",
                "white",
            ][..],
        ),
    ] {
        let mut expected = Vec::new();
        for variant in variants {
            let criterion = format!("minecraft:{variant}");
            let animal = spawn_tamable_beside_alice(&mut world, entity_type, &criterion);
            world.tick(receiver.drain());
            chat_command(
                &sender,
                &receiver,
                &mut world,
                1,
                &format!("item replace entity alice weapon.mainhand with {food} 32"),
            );
            let mut clicks = 0;
            while !written_criteria(
                &world.player_advancements_to_data(ConnectionId(1)),
                advancement,
            )
            .unwrap_or_default()
            .contains(&bbb_protocol::JavaString::from(&criterion))
            {
                assert!(clicks < 32, "{criterion} tames inside 32 clicks");
                feed_tamable(&sender, &receiver, &mut world, animal);
                clicks += 1;
            }
            expected.push(criterion.into());
            assert_eq!(
                written_criteria(
                    &world.player_advancements_to_data(ConnectionId(1)),
                    advancement
                ),
                Some(expected.clone()),
                "each taming grants only the animal's saved variant"
            );
            assert_eq!(
                world.player_advancement_is_done(ConnectionId(1), advancement),
                expected.len() == variants.len(),
            );
            assert!(world.player_advancement_is_done(ConnectionId(1), TAME_AN_ANIMAL));
        }
    }
    let written = world.player_advancements_to_data(ConnectionId(1));
    assert_eq!(
        written_criteria(&written, TAME_AN_ANIMAL),
        Some(vec!["tamed_animal".into()]),
    );
}

#[test]
fn player_slots_gate_tick_advancements_and_their_loot_rewards() {
    use crate::tests::entities::{console_command, console_command_answer};

    let [spawn_x, spawn_y, spawn_z] = ServerWorld::fixture_spawn_block();
    let chest_nbt = format!(
        "{{id:'minecraft:chest',x:{},y:{spawn_y},z:{spawn_z},Items:[{{Slot:0b,id:'minecraft:diamond',count:3}}]}}",
        spawn_x + 1
    );
    let slots = serde_json::json!({
        "inventory.0":{"items":"minecraft:diamond", "components":{"minecraft:custom_data":{"marked":17}}},
        "player.cursor":{"items":"minecraft:emerald", "count":3},
        "player.crafting.0":{"items":"minecraft:iron_ingot", "count":2},
        "enderchest.26":{"items":"minecraft:gold_ingot", "count":7},
        "player.crafting.1":{"count":0}
    });
    let reward = serde_json::json!({"type":"minecraft:advancement_reward","pools":[
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"slots":slots,"team":"hunters"}},
            {"condition":"minecraft:location_check","offsetX":1,
                "predicate":{"block":{"nbt":chest_nbt}}}],"entries":[{"type":"minecraft:item","name":"minecraft:lapis_lazuli"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"slots":{"enderchest.26":{"items":"minecraft:diamond"}}}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:netherite_ingot"}]},
        {"rolls":1,"conditions":[{"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"team":"gatherers"}}],
            "entries":[{"type":"minecraft:item","name":"minecraft:copper_ingot"}]}
    ]});
    let config = datapack_config_with_loot_table("minecraft:entities/zombie", &reward);
    let mut startup = (**config.datapacks.as_ref().unwrap()).clone();
    let player_conditions = serde_json::json!([
        {"condition":"minecraft:entity_properties","entity":"this",
            "predicate":{"slots":slots,"team":"hunters"}},
        {"condition":"minecraft:location_check","offsetX":1,
            "predicate":{"block":{"nbt":chest_nbt},"biomes":"minecraft:snowy_plains"}},
        {"condition":"minecraft:entity_scores","entity":"this","scores":{"points":4}},
        {"condition":"minecraft:value_check",
            "value":{"type":"minecraft:score","target":{"type":"minecraft:fixed","name":"global"},"score":"points"},
            "range":9},
        {"condition":"minecraft:time_check","clock":"minecraft:the_end","value":107},
        {"condition":"minecraft:environment_attribute_check",
            "attribute":"minecraft:gameplay/water_evaporates","value":false}
    ]);
    let mut criteria = vec![
        ("test:player_slots", player_conditions.clone()),
        (
            "test:wrong_player_slots",
            serde_json::json!({"slots":{"enderchest.26":{"items":"minecraft:diamond"}}}),
        ),
    ];
    for (id, index, replacement) in [
        (
            "test:wrong_tick_team",
            0,
            serde_json::json!({"condition":"minecraft:entity_properties","entity":"this","predicate":{"team":"gatherers"}}),
        ),
        (
            "test:wrong_tick_biome",
            1,
            serde_json::json!({"condition":"minecraft:location_check","predicate":{"biomes":"minecraft:desert"}}),
        ),
        (
            "test:wrong_tick_entity_score",
            2,
            serde_json::json!({"condition":"minecraft:entity_scores","entity":"this","scores":{"points":5}}),
        ),
        (
            "test:wrong_tick_named_score",
            3,
            serde_json::json!({"condition":"minecraft:value_check","value":{"type":"minecraft:score","target":{"type":"minecraft:fixed","name":"global"},"score":"points"},"range":10}),
        ),
        (
            "test:wrong_tick_clock",
            4,
            serde_json::json!({"condition":"minecraft:time_check","clock":"minecraft:the_end","value":108}),
        ),
        (
            "test:wrong_tick_environment",
            5,
            serde_json::json!({"condition":"minecraft:environment_attribute_check","attribute":"minecraft:gameplay/water_evaporates","value":true}),
        ),
    ] {
        let mut predicate = player_conditions.clone();
        predicate[index] = replacement;
        criteria.push((id, predicate));
    }
    for (id, predicate) in criteria {
        startup
            .advancements
            .advancements
            .push(bbb_server_datapack::advancement::AdvancementElement {
            id: id.into(),
            source_pack: "test".into(),
            advancement: crate::tests::parse_advancement(&serde_json::json!({
                "criteria":{"tick":{"trigger":"minecraft:tick","conditions":{"player":predicate}}},
                "rewards":{"loot":["minecraft:entities/zombie"]}
            }))
            .unwrap(),
        });
    }
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.datapacks = Some(Arc::new(startup));
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:player_slots"));
    let position = world.publication().players[0].position;
    let chest = (
        position.x.floor() as i32 + 1,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    assert_eq!(chest, (spawn_x + 1, spawn_y, spawn_z));
    let at = format!("{} {} {}", chest.0, chest.1, chest.2);
    for command in [
        "team add hunters".to_string(),
        "team join hunters alice".to_string(),
        format!("setblock {at} chest"),
        format!("item replace block {at} container.0 with diamond 3"),
    ] {
        let result = console_command(&sender, &receiver, &mut world, 1, &command);
        assert_eq!(console_command_answer(&result).0, Some(1), "{command}");
    }
    for command in [
        "scoreboard objectives add points dummy".to_string(),
        "scoreboard players set alice points 4".to_string(),
        "scoreboard players set global points 9".to_string(),
        "time of minecraft:the_end pause".to_string(),
        "time of minecraft:the_end set 107".to_string(),
        format!(
            "fillbiome {} {} {} {} {} {} minecraft:snowy_plains",
            spawn_x - 8,
            spawn_y - 8,
            spawn_z - 8,
            spawn_x + 8,
            spawn_y + 8,
            spawn_z + 8
        ),
    ] {
        let result = console_command(&sender, &receiver, &mut world, 1, &command);
        assert!(console_command_answer(&result).0.is_some(), "{command}");
    }
    for command in [
        "item replace entity alice inventory.0 with diamond[custom_data={marked:17b}]",
        "item replace entity alice player.cursor with emerald 3",
        "item replace entity alice player.crafting.0 with iron_ingot 2",
        "item replace entity alice enderchest.26 with gold_ingot 7",
    ] {
        chat_command(&sender, &receiver, &mut world, 1, command);
    }
    world.tick(Vec::new());
    assert!(world.player_advancement_is_done(ConnectionId(1), "test:player_slots"));
    assert!(!world.player_advancement_is_done(ConnectionId(1), "test:wrong_player_slots"));
    for id in [
        "test:wrong_tick_team",
        "test:wrong_tick_biome",
        "test:wrong_tick_entity_score",
        "test:wrong_tick_named_score",
        "test:wrong_tick_clock",
        "test:wrong_tick_environment",
    ] {
        assert!(
            !world.player_advancement_is_done(ConnectionId(1), id),
            "{id}"
        );
    }
    let inventory = &world.publication().players[0].inventory.slots;
    let count = |id| {
        inventory
            .iter()
            .filter(|stack| stack.item_id == Some(id))
            .map(|stack| stack.count)
            .sum::<i32>()
    };
    assert_eq!(
        count(bbb_protocol::item_types::VANILLA_ITEM_LAPIS_LAZULI_ID),
        1
    );
    assert_eq!(
        count(bbb_protocol::item_types::VANILLA_ITEM_NETHERITE_INGOT_ID),
        0
    );
    assert_eq!(
        count(bbb_protocol::item_types::VANILLA_ITEM_COPPER_INGOT_ID),
        0
    );
}
