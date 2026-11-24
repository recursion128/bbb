use std::sync::Arc;

use crate::tests::parse_advancement;
use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
use serde_json::json;

use super::*;
use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
use crate::world::ServerWorldConfig;

#[test]
fn location_uses_the_live_player_position_and_dimension_and_rewards_once() {
    let mut startup = DatapackStartup::load(&DatapackStartupOptions {
        datapacks_dir: None,
        saved_data_packs: None,
        safe_mode: false,
        limits: Default::default(),
    })
    .unwrap();
    startup.advancements = AdvancementSet {
        advancements: vec![AdvancementElement {
            id: "test:high_up".into(),
            source_pack: "test".into(),
            advancement: parse_advancement(&json!({
                "criteria": {"located": {
                    "trigger": "minecraft:location",
                    "conditions": {"player": {
                        "team": "red",
                        "location": {"dimension": "minecraft:the_nether", "position": {"y": {"min": 100}}},
                        "nbt": "{SelectedItem:{id:\"minecraft:diamond\",count:1}}"
                    }}
                }},
                "rewards": {"experience": 7}
            }))
            .unwrap(),
        }],
        warnings: Vec::new(),
    };
    let mut world = ServerWorld::new(ServerWorldConfig {
        datapacks: Some(Arc::new(startup)),
        ..Default::default()
    })
    .unwrap();
    let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
    crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "explorer", 1);
    let player = ConnectionId(1);
    for (request_id, command) in [
        (2, "team add red"),
        (3, "team add blue"),
        (4, "team join red explorer"),
    ] {
        crate::tests::console_command(&sender, &receiver, &mut world, request_id, command);
    }
    world.set_inventory_slot_for_test(
        player,
        0,
        crate::entity::item::plain_stack("minecraft:diamond", 1)
            .unwrap()
            .into_summary(),
    );
    let mut position = world.levels[player].position();
    position.y = 64.0;
    world.levels[player].set_position(position);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(!world.player_advancement_is_done(player, "test:high_up"));
    assert_eq!(world.levels[player].experience.total(), 0);
    position.y = 110.0;
    world.levels[player].set_position(position);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(!world.player_advancement_is_done(player, "test:high_up"));
    world.set_inventory_slot_for_test(
        player,
        0,
        crate::entity::item::plain_stack("minecraft:stone", 1)
            .unwrap()
            .into_summary(),
    );
    assert!(world.teleport_player(
        player,
        crate::world::TeleportTransition {
            new_level: bbb_server_types::save::Java26BuiltinDimension::Nether.into(),
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            relatives_mask: 0,
        },
        &mut Vec::new(),
    ));
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(!world.player_advancement_is_done(player, "test:high_up"));
    assert_eq!(world.levels[player].experience.total(), 0);
    crate::tests::console_command(&sender, &receiver, &mut world, 5, "team join blue explorer");
    world.set_inventory_slot_for_test(
        player,
        0,
        crate::entity::item::plain_stack("minecraft:diamond", 1)
            .unwrap()
            .into_summary(),
    );
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert!(!world.player_advancement_is_done(player, "test:high_up"));
    assert_eq!(world.levels[player].experience.total(), 0);
    crate::tests::console_command(&sender, &receiver, &mut world, 6, "team join red explorer");
    for _ in 0..40 {
        world.tick(Vec::new());
        if world.player_advancement_is_done(player, "test:high_up") {
            break;
        }
    }
    assert!(world.player_advancement_is_done(player, "test:high_up"));
    assert_eq!(world.levels[player].experience.total(), 7);
    for _ in 0..40 {
        world.tick(Vec::new());
    }
    assert_eq!(world.levels[player].experience.total(), 7);
}

mod biome_tags;
mod lava_ride;
mod light;
mod structures;
