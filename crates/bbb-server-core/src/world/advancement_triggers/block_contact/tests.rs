use std::sync::{Arc, OnceLock};

use crate::tests::parse_advancement;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::packets::{GameProfile, Vec3d};
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::player_advancements::PlayerAdvancements;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use bbb_server_persistence::Java26PlayerAdvancements;
use serde_json::{json, Value};
use uuid::Uuid;

use super::*;
use crate::world::{JoinContext, ServerWorldConfig};

const PLAYER: ConnectionId = ConnectionId(1);

fn startup(definitions: &[(&str, Value)]) -> Arc<DatapackStartup> {
    static BUILTIN: OnceLock<DatapackStartup> = OnceLock::new();
    let mut startup = BUILTIN
        .get_or_init(|| {
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap()
        })
        .clone();
    startup.advancements = AdvancementSet {
        advancements: definitions
            .iter()
            .map(|(id, value)| AdvancementElement {
                id: (*id).into(),
                source_pack: "test".into(),
                advancement: parse_advancement(value).unwrap(),
            })
            .collect(),
        warnings: Vec::new(),
    };
    Arc::new(startup)
}

fn world(definitions: &[(&str, Value)]) -> ServerWorld {
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(startup(definitions)),
        ..Default::default()
    })
    .unwrap();
    world.join(
        PLAYER,
        GameProfile {
            uuid: Uuid::from_u128(1),
            name: "slider".into(),
            properties: Vec::new(),
        },
        Default::default(),
        JoinContext::default(),
        &mut Vec::new(),
    );
    world.levels[PLAYER].set_position(Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    });
    world
}

fn advancement(conditions: Value, rewards: Value) -> Value {
    json!({"criteria":{"slide":{"trigger":"slide_down_block","conditions":conditions}},"rewards":rewards})
}

fn state(block: &str, properties: &[(&str, &str)]) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        block,
        &properties
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
    )
    .unwrap()
}

fn fire(world: &mut ServerWorld, state: i32) {
    world.trigger_slide_down_block(PLAYER, state, &mut Vec::new());
}

fn done(world: &ServerWorld, id: &str) -> bool {
    world.player_advancement_is_done(PLAYER, id)
}
fn xp(world: &ServerWorld) -> i32 {
    world.levels[PLAYER].experience.total()
}

#[test]
fn slide_down_block_player_object_and_array_use_real_session_and_origin() {
    let predicate = json!({"type":"minecraft:player","distance":{"absolute":{"max":0}},"location":{"position":{"y":64}},"flags":{"is_on_ground":true}});
    let mut world = world(&[
        (
            "test:object",
            advancement(
                json!({"block":"honey_block","player":predicate}),
                json!({"experience":2}),
            ),
        ),
        (
            "test:array",
            advancement(
                json!({"player":[{"condition":"entity_properties","entity":"this","predicate":predicate}]}),
                json!({"experience":3}),
            ),
        ),
        (
            "test:null",
            advancement(json!({"player":null}), json!({"experience":5})),
        ),
    ]);
    world.levels[PLAYER].set_on_ground(false);
    fire(&mut world, state("minecraft:honey_block", &[]));
    assert_eq!(xp(&world), 5);
    world.levels[PLAYER].set_on_ground(true);
    fire(&mut world, state("minecraft:honey_block", &[]));
    assert_eq!(xp(&world), 10);
    assert!(done(&world, "test:object") && done(&world, "test:array"));
}

#[test]
fn slide_down_block_state_ranges_use_typed_property_comparison() {
    let mut world = world(&[
        (
            "test:range",
            advancement(
                json!({"block":"wheat","state":{"age":{"min":"+02","max":"5"}}}),
                json!({"experience":2}),
            ),
        ),
        (
            "test:exact",
            advancement(json!({"state":{"age":"+02"}}), json!({"experience":3})),
        ),
    ]);
    fire(&mut world, state("minecraft:stone", &[]));
    fire(&mut world, state("minecraft:wheat", &[("age", "1")]));
    assert_eq!(xp(&world), 0);
    fire(&mut world, state("minecraft:wheat", &[("age", "2")]));
    assert_eq!(xp(&world), 5);
}

#[test]
fn slide_down_block_canonical_false_is_not_an_absent_flag() {
    let mut world = world(&[(
        "test:false",
        advancement(
            json!({"player":{"flags":{"is_on_ground":false}}}),
            json!({"experience":3}),
        ),
    )]);
    world.levels[PLAYER].set_on_ground(true);
    fire(&mut world, state("minecraft:honey_block", &[]));
    assert!(!done(&world, "test:false"));
    world.levels[PLAYER].set_on_ground(false);
    fire(&mut world, state("minecraft:honey_block", &[]));
    assert!(done(&world, "test:false"));
    assert_eq!(xp(&world), 3);
}

#[test]
fn slide_down_block_awards_once_and_reload_save_revoke_preserve_listeners() {
    let definitions = [(
        "test:slide",
        advancement(json!({"block":"honey_block"}), json!({"experience":7})),
    )];
    let mut world = world(&definitions);
    let honey = state("minecraft:honey_block", &[]);
    fire(&mut world, honey);
    fire(&mut world, honey);
    assert_eq!(xp(&world), 7);
    assert_eq!(
        world.levels[PLAYER]
            .cold
            .advancements
            .trigger_listeners(SLIDE_DOWN_BLOCK)
            .count(),
        0
    );
    let data = world.levels[PLAYER]
        .cold
        .advancements
        .to_data(&world.cold.advancement_tree);
    let saved = Java26PlayerAdvancements::parse(&data.to_document()).unwrap();
    world.levels[PLAYER].cold.advancements =
        PlayerAdvancements::load(&world.cold.advancement_tree, &saved);
    world.reload_player_advancement_stores();
    fire(&mut world, honey);
    assert_eq!(xp(&world), 7);
    let index = world.cold.advancement_tree.index_of("test:slide").unwrap();
    assert!(world.levels[PLAYER].cold.advancements.revoke(
        &world.cold.advancement_tree,
        index,
        &"slide".into()
    ));
    world.cold.config.datapacks = Some(startup(&[(
        "test:slide",
        advancement(json!({"block":"stone"}), json!({"experience":7})),
    )]));
    world.reload_player_advancement_stores();
    fire(&mut world, honey);
    assert_eq!(xp(&world), 7);
    fire(&mut world, state("minecraft:stone", &[]));
    assert_eq!(xp(&world), 14);
}

mod enter_block;
