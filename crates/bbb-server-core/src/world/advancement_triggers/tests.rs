use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use crate::connection::ClientboundPlayPacket;
use crate::entity::{EntityData, GeneralLivingData};
use crate::tests::parse_advancement;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};
use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
use bbb_protocol::packets::{
    BlockHitResult, Direction, GameProfile, InteractionHand, PlayServerbound, UseItemOn,
};
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::startup::DatapackStartupOptions;
use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::admin::{NameAndId, OperatorEntry};
use crate::commands::CommandPermission;
use crate::world::{JoinContext, ServerWorldConfig, WorldCommand};

const CONNECTION: ConnectionId = ConnectionId(1);
const VICTIM: ConnectionId = ConnectionId(2);

mod avoid_vibration;
mod block_use;
mod captured_custom_data;
mod captured_identity;
mod captured_nbt;
mod criterion_validation;
mod effects_and_levitation;
use effects_and_levitation::effect;
mod entity_clocks;
mod entity_scores;
mod explosion_fall;
mod item_use_sites;
mod kills;
mod lifecycle;
mod summoned;
mod vehicle_slots;

fn built_in_startup() -> &'static Arc<DatapackStartup> {
    static BUILTIN: OnceLock<Arc<DatapackStartup>> = OnceLock::new();
    BUILTIN.get_or_init(|| {
        Arc::new(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap(),
        )
    })
}

fn startup(definitions: &[(&str, Value)]) -> Arc<DatapackStartup> {
    let mut startup = (**built_in_startup()).clone();
    let mut advancements = definitions
        .iter()
        .map(|(id, body)| AdvancementElement {
            id: (*id).into(),
            source_pack: "test".into(),
            advancement: parse_advancement(body).unwrap(),
        })
        .collect::<Vec<_>>();
    advancements.sort_by(|left, right| left.id.cmp(&right.id));
    startup.advancements = AdvancementSet {
        advancements,
        warnings: Vec::new(),
    };
    Arc::new(startup)
}

fn world(definitions: &[(&str, Value)]) -> ServerWorld {
    let user = NameAndId {
        id: Uuid::from_u128(1),
        name: "inventory_player".into(),
    };
    let mut config = ServerWorldConfig {
        datapacks: Some(startup(&[])),
        ..Default::default()
    };
    config.admin_lists.operators.insert(
        user.id,
        OperatorEntry {
            user: user.clone(),
            level: CommandPermission::Owner,
            bypasses_player_limit: false,
        },
    );
    let mut world = ServerWorld::new(config).unwrap();
    world.join(
        CONNECTION,
        GameProfile {
            uuid: user.id,
            name: user
                .name
                .try_to_string()
                .expect("fixture login name is valid UTF-8"),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    assert!(world.levels.contains_player(CONNECTION));
    world.levels[CONNECTION]
        .inventory
        .clear_all_items(None)
        .expect("the fixture inventory clears");
    world.broadcast_player_menu(CONNECTION, MenuBroadcastMode::Changes, &mut Vec::new());
    world.cold.config.datapacks = Some(startup(definitions));
    world.reload_player_advancement_stores();
    world
}

fn built_in_world() -> ServerWorld {
    let mut world = world(&[]);
    world.cold.config.datapacks = Some(Arc::clone(built_in_startup()));
    world.reload_player_advancement_stores();
    world
}

fn kill_world(definitions: &[(&str, Value)]) -> ServerWorld {
    let mut world = world(definitions);
    world.join(
        VICTIM,
        GameProfile {
            uuid: Uuid::from_u128(2),
            name: "victim_player".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    command(&mut world, "gamemode survival inventory_player");
    command(&mut world, "gamemode survival victim_player");
    for connection_id in [CONNECTION, VICTIM] {
        send_play(&mut world, connection_id, PlayServerbound::PlayerLoaded);
    }
    world
}

fn send_play(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    packet: PlayServerbound,
) -> Vec<WorldEvent> {
    world
        .tick(vec![crate::mailbox::CommandEnvelope::for_test(
            world.last_command_sequence + 1,
            crate::mailbox::WorldProducerClass::Network,
            WorldCommand::Play {
                connection_id,
                packet,
            },
        )])
        .events
}

fn assert_advancements(
    world: &ServerWorld,
    connection_id: ConnectionId,
    done: &[&str],
    not_done: &[&str],
) {
    for &id in done {
        assert!(
            world.player_advancement_is_done(connection_id, id),
            "expected {id} done"
        );
    }
    for &id in not_done {
        assert!(
            !world.player_advancement_is_done(connection_id, id),
            "expected {id} not done"
        );
    }
}

fn compiled_criterion_ids<T>(tree: &AdvancementTree, compiled: &Compiled<T>) -> Vec<String> {
    let mut ids = Vec::new();
    for (index, criteria) in compiled {
        for name in criteria.keys() {
            ids.push(format!("{}/{name}", tree.node(*index).id()));
        }
    }
    ids.sort();
    ids
}

fn inventory(conditions: Value, rewards: Value) -> Value {
    json!({"criteria":{"done":{"trigger":"minecraft:inventory_changed","conditions":conditions}},"rewards":rewards})
}

fn command(world: &mut ServerWorld, command: &str) -> Vec<WorldEvent> {
    send_play(
        world,
        CONNECTION,
        PlayServerbound::ChatCommand {
            command: command.into(),
        },
    )
}

fn give(world: &mut ServerWorld, item: &str, count: i32) -> Vec<WorldEvent> {
    command(world, &format!("give @s {item} {count}"))
}

fn diamond_chest(world: &mut ServerWorld, pos: BlockPos) {
    let events = command(
        world,
        &format!(
            "setblock {} {} {} chest{{Items:[{{Slot:0b,id:\"minecraft:diamond\",count:3}}]}}",
            pos.x, pos.y, pos.z
        ),
    );
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::SystemChat(chat) }
            if chat.content.text().contains("commands.setblock.success")
    )), "{events:?}");
}

fn xp(world: &ServerWorld) -> i32 {
    world.levels[CONNECTION].experience.total()
}

fn tool(item: &str, count: i32) -> LootTool {
    LootTool {
        stack: Default::default(),
        item_id: Some(vanilla_item_type_for_resource_id(item).unwrap().id),
        count,
        enchantments: BTreeMap::new(),
    }
}

fn tags() -> ItemTagIndex {
    ItemTagIndex::from_loaded(startup(&[]).tags.registry("minecraft:item").unwrap())
}

fn this_entity_properties(predicate: Value) -> Value {
    json!({"condition":"minecraft:entity_properties","entity":"this","predicate":predicate})
}

fn killed(trigger: &str, conditions: Value) -> Value {
    json!({"criteria":{"killed":{"trigger":trigger,"conditions":conditions}}})
}

fn spawn_mob(world: &mut ServerWorld, entity_type_id: i32) -> (i32, Uuid) {
    let position = world.levels[CONNECTION].position();
    world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        entity_type_id,
        position,
        EntityData::GeneralLiving(
            GeneralLivingData::new(entity_type_id).expect("the type is a living entity"),
        ),
    )
}

fn kill_with(
    world: &mut ServerWorld,
    victim: &str,
    damage_type: &str,
    direct: &str,
    causing: Option<&str>,
) {
    let source = match causing {
        None => format!("by {direct}"),
        Some(causing) => format!("by {direct} from {causing}"),
    };
    command(world, &format!("damage {victim} 40 {damage_type} {source}"));
}

fn load_effect_source_nbt(world: &mut ServerWorld, id: i32, fields: &str) {
    let dimension = Java26BuiltinDimension::Overworld;
    let Some(bbb_protocol::NbtSummaryValue::Compound(mut entries)) =
        world.stored_entity_command_data_for_test(dimension, id)
    else {
        panic!("the source entity has a saved record")
    };
    let bbb_protocol::NbtSummaryValue::Compound(fields) =
        bbb_protocol::parse_snbt_compound(fields).unwrap()
    else {
        unreachable!()
    };
    for field in fields {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    world.load_stored_entity_record_for_test(dimension, id, &entries);
}

fn remote_potion_owner(world: &mut ServerWorld) -> (Uuid, i32) {
    command(world, "tp @s 8.5 80 8.5");
    command(world, "gamemode survival @s");
    player_loaded(world);
    command(world, "fill 4 79 4 12 79 12 minecraft:stone");
    let (owner, uuid) = spawn_mob(world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    load_effect_source_nbt(world, owner, "{NoAI:1b,NoGravity:1b,Motion:[0d,0d,0d]}");
    command(
        world,
        &format!("execute in minecraft:the_nether run tp {uuid} 1000.5 80 8.5"),
    );
    let dimension = world.levels[CONNECTION].dimension;
    let (owner_dimension, owner_id) = world.entity_reference(&dimension, uuid).unwrap();
    assert_eq!(
        owner_dimension,
        world.require_level_id(&Java26BuiltinDimension::Nether.into())
    );
    (uuid, owner_id)
}

fn owned_harming_cloud(world: &mut ServerWorld, owner: Uuid) -> i32 {
    let (cloud, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        world.levels[CONNECTION].position(),
        EntityData::AreaEffectCloud(
            crate::entity::area_effect_cloud::AreaEffectCloudData::new().with_owner(Some(owner)),
        ),
    );
    load_effect_source_nbt(
        world,
        cloud,
        r#"{WaitTime:0,Duration:100,Radius:1f,RadiusOnUse:-1f,potion_contents:{potion:"minecraft:harming"}}"#,
    );
    cloud
}

#[test]
fn inventory_advancement_give_reenters_recipe_rewards_once_in_packet_order() {
    let mut world = world(&[
        (
            "test:inventory",
            inventory(
                json!({"items":[{"items":"minecraft:oak_log"}]}),
                json!({"experience":2,"recipes":["minecraft:oak_planks"]}),
            ),
        ),
        (
            "test:recipe",
            json!({"criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":"minecraft:oak_planks","player":{"flags":{"is_sneaking":false}}}}},"rewards":{"experience":3,"recipes":["minecraft:stick"]}}),
        ),
        (
            "test:recipe_wrong_player",
            json!({"criteria":{"done":{"trigger":"minecraft:recipe_unlocked","conditions":{"recipe":"minecraft:oak_planks","player":{"flags":{"is_sneaking":true}}}}},"rewards":{"experience":99}}),
        ),
    ]);
    let events = give(&mut world, "minecraft:oak_log", 1);
    assert_eq!(xp(&world), 5);
    assert!(!world.player_advancement_is_done(CONNECTION, "test:recipe_wrong_player"));
    let recipes = world.levels[CONNECTION].inventory.recipe_book();
    assert!(recipes.contains("minecraft:oak_planks"));
    assert!(recipes.contains("minecraft:stick"));
    let packets = events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                connection_id: CONNECTION,
                packet: crate::connection::ClientboundPlayPacket::RecipeBookAdd(packet),
            } => Some(packet),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(packets.len(), 2);
    for (packet, recipe) in packets
        .iter()
        .zip(["minecraft:stick", "minecraft:oak_planks"])
    {
        let displays = world
            .cold
            .config
            .datapacks
            .as_ref()
            .unwrap()
            .recipe_displays
            .displays_for_recipe(recipe);
        assert_eq!(
            packet
                .entries
                .iter()
                .map(|entry| &entry.contents)
                .collect::<Vec<_>>(),
            displays.iter().collect::<Vec<_>>()
        );
    }
    give(&mut world, "minecraft:oak_log", 1);
    assert_eq!(xp(&world), 5);
}

#[test]
fn inventory_advancement_without_item_predicates_uses_full_and_occupied_slots() {
    let mut world = world(&[(
        "test:slots",
        inventory(
            json!({"slots":{"occupied":1,"full":1}}),
            json!({"experience":7}),
        ),
    )]);
    assert_eq!(xp(&world), 0);
    give(&mut world, "minecraft:stone", 1);
    assert_eq!(xp(&world), 0);
    give(&mut world, "minecraft:stone", 63);
    assert_eq!(xp(&world), 7);
}

#[test]
fn inventory_advancement_single_predicate_only_observes_changed_nonempty_stack() {
    let condition = parse_inventory_changed_condition(
        &bbb_server_datapack::loot::codec::registry_json(
            &json!({"items":[{"items":"minecraft:stone"}]}),
        )
        .unwrap(),
    )
    .unwrap();
    let stone = tool("minecraft:stone", 1);
    let dirt = tool("minecraft:dirt", 1);
    let both = [stone.clone(), dirt.clone()];
    let only_stone = [stone.clone()];
    let tags = tags();
    let resolver = crate::world::registry_runtime::item_stack_resolver_for_test();
    for (counts, stacks, changed, changed_nonempty, expected) in [
        ([2, 0, 34], &both[..], &dirt, true, false),
        ([2, 0, 34], &both[..], &stone, true, true),
        ([1, 0, 35], &only_stone[..], &stone, false, false),
    ] {
        assert_eq!(
            inventory_condition_matches(
                &condition,
                counts,
                stacks,
                changed,
                changed_nonempty,
                &tags,
                resolver
            )
            .unwrap(),
            expected
        );
    }
}

#[test]
fn inventory_advancement_multiple_predicates_can_share_one_inventory_stack() {
    let condition = parse_inventory_changed_condition(
        &bbb_server_datapack::loot::codec::registry_json(
            &json!({"items":[{"items":"minecraft:oak_log"},{"items":"#minecraft:logs"}]}),
        )
        .unwrap(),
    )
    .unwrap();
    let log = tool("minecraft:oak_log", 1);
    let dirt = tool("minecraft:dirt", 1);
    let tags = tags();
    let resolver = crate::world::registry_runtime::item_stack_resolver_for_test();
    assert!(inventory_condition_matches(
        &condition,
        [2, 0, 34],
        &[log, dirt.clone()],
        &dirt,
        true,
        &tags,
        resolver
    )
    .unwrap());
    assert!(!inventory_condition_matches(
        &condition,
        [1, 0, 35],
        std::slice::from_ref(&dirt),
        &dirt,
        true,
        &tags,
        resolver
    )
    .unwrap());
}

#[test]
fn inventory_advancement_give_matches_loaded_item_tag() {
    let mut world = world(&[(
        "test:tag",
        inventory(
            json!({"items":[{"items":"#minecraft:logs"}]}),
            json!({"experience":11}),
        ),
    )]);
    give(&mut world, "minecraft:stone", 1);
    assert_eq!(xp(&world), 0);
    give(&mut world, "minecraft:oak_log", 1);
    assert_eq!(xp(&world), 11);
}

#[test]
fn inventory_advancement_player_object_and_list_read_the_actual_pose() {
    let predicate = json!({"type":"minecraft:player", "flags":{"is_sneaking":true},
        "nbt":"{SelectedItem:{id:\"minecraft:diamond\",count:1}}"});
    for player in [
        predicate.clone(),
        json!([this_entity_properties(predicate)]),
    ] {
        let mut world = world(&[(
            "test:player",
            inventory(
                json!({"player":player,"items":[{"items":"minecraft:stone"}]}),
                json!({"experience":13}),
            ),
        )]);
        let hold = |world: &mut ServerWorld, item: &str| {
            world.levels[CONNECTION].inventory.set_item_in_hand(
                InteractionHand::MainHand,
                crate::entity::item::plain_stack(item, 1).unwrap(),
            );
        };
        hold(&mut world, "minecraft:diamond");
        give(&mut world, "minecraft:stone", 1);
        assert_eq!(xp(&world), 0);
        world.levels[CONNECTION].set_shared_flag(
            bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
            true,
        );
        world.levels[CONNECTION].abilities.flying = true;
        world.tick_player_poses();
        give(&mut world, "minecraft:stone", 1);
        assert_eq!(xp(&world), 0);
        world.levels[CONNECTION].abilities.flying = false;
        world.tick_player_poses();
        hold(&mut world, "minecraft:stone");
        give(&mut world, "minecraft:stone", 1);
        assert_eq!(xp(&world), 0);
        hold(&mut world, "minecraft:diamond");
        give(&mut world, "minecraft:dirt", 1);
        assert_eq!(xp(&world), 0);
        give(&mut world, "minecraft:stone", 1);
        assert_eq!(xp(&world), 13);
        give(&mut world, "minecraft:stone", 1);
        assert_eq!(xp(&world), 13);
    }
}

#[test]
fn interaction_advancement_sneaking_reads_the_actual_player_pose() {
    let mut world = world(&[(
        "test:player",
        json!({"criteria":{"done":{"trigger":"minecraft:player_interacted_with_entity",
            "conditions":{"player":{"flags":{"is_sneaking":true},
                "nbt":"{SelectedItem:{id:\"minecraft:diamond\",count:1}}"}}}},
            "rewards":{"experience":13}}),
    )]);
    let hold = |world: &mut ServerWorld, item: &str| {
        world.levels[CONNECTION].inventory.set_item_in_hand(
            InteractionHand::MainHand,
            crate::entity::item::plain_stack(item, 1).unwrap(),
        );
    };
    hold(&mut world, "minecraft:diamond");
    let (id, _) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    let interact = |world: &mut ServerWorld| {
        let dimension = world.levels[CONNECTION].dimension;
        world.trigger_player_entity_interaction(
            CONNECTION,
            "minecraft:player_interacted_with_entity",
            dimension,
            id,
            RuntimeItemStack::empty().summary(),
            &mut Vec::new(),
        );
    };
    interact(&mut world);
    assert_eq!(xp(&world), 0);
    world.levels[CONNECTION].set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        true,
    );
    world.levels[CONNECTION].abilities.flying = true;
    world.tick_player_poses();
    interact(&mut world);
    assert_eq!(xp(&world), 0);
    world.levels[CONNECTION].abilities.flying = false;
    world.tick_player_poses();
    hold(&mut world, "minecraft:stone");
    interact(&mut world);
    assert_eq!(xp(&world), 0);
    hold(&mut world, "minecraft:diamond");
    interact(&mut world);
    assert_eq!(xp(&world), 13);
    interact(&mut world);
    assert_eq!(xp(&world), 13);
}

#[test]
fn interaction_advancement_on_fire_reads_the_player_fire_clock() {
    let criterion = |burning, reward| {
        json!({
            "criteria":{"done":{"trigger":"minecraft:player_interacted_with_entity",
                "conditions":{"player":{"flags":{"is_on_fire":burning}}}}},
            "rewards":{"experience":reward}
        })
    };
    let mut world = world(&[
        ("test:unlit", criterion(false, 7)),
        ("test:lit", criterion(true, 11)),
    ]);
    let (id, _) = spawn_mob(&mut world, VANILLA_ENTITY_TYPE_SHEEP_ID);
    let interact = |world: &mut ServerWorld| {
        let dimension = world.levels[CONNECTION].dimension;
        world.trigger_player_entity_interaction(
            CONNECTION,
            "minecraft:player_interacted_with_entity",
            dimension,
            id,
            RuntimeItemStack::empty().summary(),
            &mut Vec::new(),
        );
    };
    world.levels[CONNECTION].has_visual_fire = true;
    interact(&mut world);
    assert_eq!(xp(&world), 7);
    world.levels[CONNECTION].set_remaining_fire_ticks(40);
    interact(&mut world);
    assert_eq!(xp(&world), 18);
}

#[test]
fn inventory_advancement_reload_recompiles_conditions_without_replaying_rewards() {
    let mut world = world(&[(
        "test:reload",
        inventory(
            json!({"items":[{"items":"minecraft:stone","count":2}]}),
            json!({"experience":17}),
        ),
    )]);
    give(&mut world, "minecraft:stone", 1);
    assert_eq!(xp(&world), 0);
    world.cold.config.datapacks = Some(startup(&[(
        "test:reload",
        inventory(
            json!({"items":[{"items":"minecraft:oak_log"}]}),
            json!({"experience":19}),
        ),
    )]));
    world.reload_player_advancement_stores();
    give(&mut world, "minecraft:stone", 1);
    assert_eq!(xp(&world), 0);
    give(&mut world, "minecraft:oak_log", 1);
    assert_eq!(xp(&world), 19);
    world.reload_player_advancement_stores();
    give(&mut world, "minecraft:oak_log", 1);
    assert_eq!(xp(&world), 19);
}

fn changed_dimension(conditions: Value) -> Value {
    json!({"criteria":{"entered":{"trigger":"minecraft:changed_dimension","conditions":conditions}}})
}

fn nether_travel(conditions: Value) -> Value {
    json!({"criteria":{"travelled":{"trigger":"minecraft:nether_travel","conditions":conditions}}})
}

fn teleport(world: &mut ServerWorld, dimension: Java26BuiltinDimension, x: f64, z: f64) {
    assert!(world.teleport_player(
        CONNECTION,
        crate::world::TeleportTransition {
            new_level: dimension.into(),
            position: Vec3d { x, y: 70.0, z },
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new(),
    ));
}

#[test]
fn a_dimension_change_awards_only_the_criteria_its_two_keys_name() {
    let mut world = world(&[
        (
            "test:to_nether",
            changed_dimension(
                json!({"to":"the_nether","player":[this_entity_properties(json!({"distance":{"absolute":{"max":0}}}))]}),
            ),
        ),
        (
            "test:to_end",
            changed_dimension(json!({"to":"minecraft:the_end"})),
        ),
        (
            "test:from_nether",
            changed_dimension(json!({"from":"minecraft:the_nether"})),
        ),
        ("test:any", changed_dimension(json!({}))),
        (
            "test:wrong_player",
            changed_dimension(json!({"to":"the_nether","player":{"type":"minecraft:zombie"}})),
        ),
    ]);
    teleport(&mut world, Java26BuiltinDimension::Nether, 0.5, 0.5);
    assert_advancements(
        &world,
        CONNECTION,
        &["test:to_nether", "test:any"],
        &["test:wrong_player", "test:to_end", "test:from_nether"],
    );
    teleport(&mut world, Java26BuiltinDimension::Overworld, 0.5, 0.5);
    assert_advancements(&world, CONNECTION, &["test:from_nether"], &["test:to_end"]);
}

/// `ServerGamePacketListenerImpl.java:1870` names END to OVERWORLD whatever
/// level the credits respawn lands in.
#[test]
fn leaving_the_credits_is_always_an_end_to_overworld_change() {
    let mut world = world(&[
        (
            "test:end_to_overworld",
            changed_dimension(json!({"from":"minecraft:the_end","to":"minecraft:overworld"})),
        ),
        (
            "test:from_overworld",
            changed_dimension(json!({"from":"minecraft:overworld"})),
        ),
    ]);
    world.levels[CONNECTION].cold.won_game = true;
    world.apply_client_command(
        CONNECTION,
        bbb_protocol::packets::ClientCommandAction::PerformRespawn,
        &mut Vec::new(),
    );
    assert_advancements(
        &world,
        CONNECTION,
        &["test:end_to_overworld"],
        &["test:from_overworld"],
    );
}

/// `:1095-1100` returns before `:1123`, so a transition naming the level the
/// session is already in is not a dimension change.
#[test]
fn a_same_level_teleport_is_not_a_dimension_change() {
    let mut world = world(&[("test:any", changed_dimension(json!({})))]);
    teleport(&mut world, Java26BuiltinDimension::Overworld, 40.5, 40.5);
    assert!(!world.player_advancement_is_done(CONNECTION, "test:any"));
}

#[test]
fn nether_travel_measures_the_overworld_position_the_session_left_from() {
    let mut world = world(&[
        (
            "test:far",
            nether_travel(
                json!({"distance":{"horizontal":{"min":100.0}},"player":{"distance":{"absolute":{"max":0}}}}),
            ),
        ),
        (
            "test:farther",
            nether_travel(json!({"distance":{"horizontal":{"min":400.0}}})),
        ),
        (
            "test:wrong_player",
            nether_travel(
                json!({"distance":{"horizontal":{"min":100.0}},"player":{"type":"minecraft:zombie"}}),
            ),
        ),
    ]);
    let start = world.levels[CONNECTION].position();
    teleport(&mut world, Java26BuiltinDimension::Nether, 4.5, 4.5);
    assert_eq!(
        world.levels[CONNECTION].cold.entered_nether_position,
        Some(start)
    );
    teleport(
        &mut world,
        Java26BuiltinDimension::Overworld,
        start.x + 200.0,
        start.z,
    );
    assert_advancements(
        &world,
        CONNECTION,
        &["test:far"],
        &["test:farther", "test:wrong_player"],
    );
    assert_eq!(world.levels[CONNECTION].cold.entered_nether_position, None);
}

/// `ServerPlayer.java:388,411`: the entry position crosses a rejoin in the
/// Nether, so the later overworld arrival still measures from it.
#[test]
fn nether_travel_measures_from_the_entry_position_saved_across_a_rejoin() {
    let mut world = world(&[(
        "test:far",
        nether_travel(json!({"distance":{"horizontal":{"min":100.0}}})),
    )]);
    let start = world.levels[CONNECTION].position();
    teleport(&mut world, Java26BuiltinDimension::Nether, 4.5, 4.5);
    let profile = world.levels[CONNECTION].profile.clone();
    world.remove_player(CONNECTION, &mut Vec::new());
    world.join(
        CONNECTION,
        profile,
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    assert_eq!(
        world.levels[CONNECTION].cold.entered_nether_position,
        Some(start)
    );
    teleport(
        &mut world,
        Java26BuiltinDimension::Overworld,
        start.x + 200.0,
        start.z,
    );
    assert!(world.player_advancement_is_done(CONNECTION, "test:far"));
}

/// `:1147` reads the two keys as well, so travel is owed only by the Nether to
/// overworld leg -- and `:1151-1153` drops the position any other arrival.
#[test]
fn only_the_nether_to_overworld_leg_owes_travel() {
    let mut world = world(&[(
        "test:travelled",
        nether_travel(json!({"distance":{"horizontal":{"min":1.0}}})),
    )]);
    let start = world.levels[CONNECTION].position();
    teleport(&mut world, Java26BuiltinDimension::Nether, 4.5, 4.5);
    teleport(
        &mut world,
        Java26BuiltinDimension::End,
        start.x + 500.0,
        start.z,
    );
    assert!(!world.player_advancement_is_done(CONNECTION, "test:travelled"));
    assert_eq!(world.levels[CONNECTION].cold.entered_nether_position, None);
    teleport(
        &mut world,
        Java26BuiltinDimension::Overworld,
        start.x + 500.0,
        start.z,
    );
    assert!(!world.player_advancement_is_done(CONNECTION, "test:travelled"));
}

/// `:53` reads the `LocationPredicate` at the overworld cell the session left
/// from, which is the position the Nether arrival recorded.
#[test]
fn a_nether_travel_start_position_reads_the_cell_the_session_left() {
    let mut world = world(&[
        (
            "test:left_high",
            nether_travel(json!({"start_position":{"position":{"y":{"min":70.0}},
                "biomes":"minecraft:snowy_plains",
                "block":{"nbt":"{id:'minecraft:chest',x:8,y:70,z:8,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}})),
        ),
        (
            "test:left_low",
            nether_travel(json!({"start_position":{"position":{"y":{"max":69.0}}}})),
        ),
        (
            "test:wrong_start_biome",
            nether_travel(json!({"start_position":{"biomes":"minecraft:plains"}})),
        ),
    ]);
    teleport(&mut world, Java26BuiltinDimension::Overworld, 8.5, 8.5);
    player_loaded(&mut world);
    diamond_chest(&mut world, BlockPos { x: 8, y: 70, z: 8 });
    let events = command(
        &mut world,
        "fillbiome 4 64 4 12 76 12 minecraft:snowy_plains",
    );
    assert!(events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: CONNECTION, packet: ClientboundPlayPacket::SystemChat(chat) }
            if chat.content.text().contains("commands.fillbiome.success")
    )), "{events:?}");
    teleport(&mut world, Java26BuiltinDimension::Nether, 4.5, 4.5);
    teleport(&mut world, Java26BuiltinDimension::Overworld, 24.5, 8.5);
    assert_advancements(
        &world,
        CONNECTION,
        &["test:left_high"],
        &["test:left_low", "test:wrong_start_biome"],
    );
}

fn placed_block(location: Value) -> Value {
    json!({"criteria":{"placed":{"trigger":"minecraft:placed_block","conditions":{"location":location}}}})
}

fn item_used_on_block(location: Value) -> Value {
    json!({"criteria":{"used":{"trigger":"minecraft:item_used_on_block","conditions":{"location":location}}}})
}

fn block_state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect();
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        name,
        &properties,
    )
    .expect("the name and its properties are a vanilla 26.1 block state")
}

fn beside_player(world: &mut ServerWorld, state: i32) -> BlockPos {
    let position = world.levels[CONNECTION].position();
    let at = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32 + 2,
    };
    let air = world
        .level(Java26BuiltinDimension::Overworld)
        .chunks
        .air_state_id();
    for offset in 1..=2 {
        world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos {
                y: at.y + offset,
                ..at
            },
            air,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
    }
    world.set_block(
        Java26BuiltinDimension::Overworld,
        at,
        state,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    at
}

fn player_loaded(world: &mut ServerWorld) {
    let teleport_id = world.levels[CONNECTION]
        .awaiting_teleport
        .expect("the join sent a teleport");
    for packet in [
        PlayServerbound::AcceptTeleportation { id: teleport_id },
        PlayServerbound::PlayerLoaded,
    ] {
        send_play(world, CONNECTION, packet);
    }
    assert!(world.levels[CONNECTION].client_loaded());
    assert_eq!(world.levels[CONNECTION].awaiting_teleport, None);
}

fn click(world: &mut ServerWorld, pos: BlockPos, sequence: i32) {
    send_play(
        world,
        CONNECTION,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::Up,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence,
        }),
    );
}

/// The two positions are different cells: `BlockItem.place` awards at the one
/// it wrote (`BlockItem.java:71`, `:82`) while the item leg that returned
/// awards at the one the click named (`ServerPlayerGameMode.java:352`, `:398`).
#[test]
fn a_placement_awards_the_written_cell_and_the_clicked_one_apart() {
    let mut world = world(&[
        (
            "test:placed_dirt",
            json!({"criteria":{"placed":{"trigger":"minecraft:placed_block","conditions":{
                "location":[{"condition":"minecraft:block_state_property","block":"minecraft:dirt"}],
                "player":{"distance":{"absolute":{"max":0}}}
            }}}}),
        ),
        (
            "test:placed_stone",
            placed_block(
                json!([{"condition":"minecraft:block_state_property","block":"minecraft:stone"}]),
            ),
        ),
        (
            "test:used_on_dirt",
            item_used_on_block(
                json!([{"condition":"minecraft:block_state_property","block":"minecraft:dirt"}]),
            ),
        ),
        (
            "test:used_on_stone",
            json!({"criteria":{"used":{"trigger":"minecraft:item_used_on_block","conditions":{
                "location":[{"condition":"minecraft:block_state_property","block":"minecraft:stone"}],
                "player":[this_entity_properties(json!({"distance":{"absolute":{"max":0}}}))]
            }}}}),
        ),
        (
            "test:wrong_player",
            json!({"criteria":{"placed":{"trigger":"minecraft:placed_block","conditions":{
                "location":[{"condition":"minecraft:block_state_property","block":"minecraft:dirt"}],
                "player":{"distance":{"absolute":{"min":1}}}
            }}}}),
        ),
    ]);
    player_loaded(&mut world);
    let at = beside_player(&mut world, block_state("minecraft:stone", &[]));
    give(&mut world, "minecraft:dirt", 1);
    click(&mut world, at, 1);
    assert_eq!(
        world.block_state(
            Java26BuiltinDimension::Overworld,
            BlockPos { y: at.y + 1, ..at }
        ),
        Some(block_state("minecraft:dirt", &[]))
    );
    assert_advancements(
        &world,
        CONNECTION,
        &["test:placed_dirt", "test:used_on_stone"],
        &[
            "test:placed_stone",
            "test:used_on_dirt",
            "test:wrong_player",
        ],
    );
}

/// `:36` reads the state after the block answered the click, and `:41` binds
/// the copy of the stack the click was entered with.
#[test]
fn the_block_leg_reads_the_state_its_own_interaction_left() {
    let mut world = world(&[
        (
            "test:charged",
            item_used_on_block(json!([
                {"condition":"minecraft:location_check","predicate":{"block":{"blocks":"minecraft:respawn_anchor","state":{"charges":"1"}}}},
                {"condition":"minecraft:match_tool","predicate":{"items":"minecraft:glowstone"}},
                {"condition":"minecraft:location_check","offsetX":1,"predicate":{"block":{"nbt":"{id:'minecraft:chest',x:9,y:70,z:10,Items:[{Slot:0b,id:'minecraft:diamond',count:3}]}"}}}
            ])),
        ),
        (
            "test:uncharged",
            item_used_on_block(
                json!([{"condition":"minecraft:location_check","predicate":{"block":{"blocks":"minecraft:respawn_anchor","state":{"charges":"0"}}}}]),
            ),
        ),
        (
            "test:wrong_tool",
            item_used_on_block(
                json!([{"condition":"minecraft:match_tool","predicate":{"items":"minecraft:stone"}}]),
            ),
        ),
    ]);
    teleport(&mut world, Java26BuiltinDimension::Overworld, 8.5, 8.5);
    player_loaded(&mut world);
    diamond_chest(&mut world, BlockPos { x: 9, y: 70, z: 10 });
    let at = beside_player(
        &mut world,
        block_state("minecraft:respawn_anchor", &[("charges", "0")]),
    );
    give(&mut world, "minecraft:glowstone", 1);
    click(&mut world, at, 1);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(block_state("minecraft:respawn_anchor", &[("charges", "1")]))
    );
    assert_advancements(
        &world,
        CONNECTION,
        &["test:charged"],
        &["test:uncharged", "test:wrong_tool"],
    );
}

/// `nether/charge_respawn_anchor` names the fourth charge, so only the click
/// that wrote it awards: `:36` reads the state the interaction left.
#[test]
fn charging_a_respawn_anchor_full_completes_the_built_in_advancement_that_names_it() {
    let mut world = built_in_world();
    player_loaded(&mut world);
    let at = beside_player(
        &mut world,
        block_state("minecraft:respawn_anchor", &[("charges", "0")]),
    );
    give(&mut world, "minecraft:glowstone", 4);
    for sequence in 1..=3 {
        click(&mut world, at, sequence);
        assert!(
            !world.player_advancement_is_done(CONNECTION, "minecraft:nether/charge_respawn_anchor")
        );
    }
    click(&mut world, at, 4);
    assert_eq!(
        world.block_state(Java26BuiltinDimension::Overworld, at),
        Some(block_state("minecraft:respawn_anchor", &[("charges", "4")]))
    );
    assert!(world.player_advancement_is_done(CONNECTION, "minecraft:nether/charge_respawn_anchor"));
}

#[test]
fn built_in_item_used_on_location_criteria_compile() {
    let startup = built_in_startup();
    let tree = AdvancementTree::from_loaded(&startup.advancements);
    let compiled = AdvancementTriggers::compile(&tree, Some(startup));
    let mut expected = BTreeSet::new();
    for element in &startup.advancements.advancements {
        let index = tree.index_of(&element.id).unwrap();
        for criterion in &element.advancement.criteria {
            if matches!(
                criterion.trigger.as_str(),
                "minecraft:placed_block"
                    | "minecraft:item_used_on_block"
                    | "minecraft:allay_drop_item_on_block"
            ) {
                expected.insert((criterion.trigger.clone(), index, criterion.name.clone()));
            }
        }
    }
    let mut actual = BTreeSet::new();
    for (trigger, criteria) in &compiled.item_used_on_location {
        for (index, named) in criteria {
            for name in named.keys() {
                actual.insert((trigger.clone(), *index, name.clone()));
            }
        }
    }
    assert_eq!(actual, expected);
}

#[test]
fn spear_mobs_rejects_invalid_count_instead_of_matching_without_a_limit() {
    let invalid = json!({"criteria":{"done":{"trigger":"minecraft:spear_mobs","conditions":{"count":"bad"}}}});
    assert!(parse_advancement(&invalid).is_err());
    let mut world = world(&[(
        "test:two",
        json!({"criteria":{"done":{"trigger":"minecraft:spear_mobs","conditions":{"count":2}}}}),
    )]);
    world.trigger_spear_mobs(CONNECTION, 2, &mut Vec::new());
    assert_advancements(&world, CONNECTION, &["test:two"], &["test:invalid"]);
}
