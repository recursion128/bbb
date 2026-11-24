//! `ShearsItem.useOn` (`ShearsItem.java:62-85`): shearing a growing plant's
//! tip stops it growing. Carving a pumpkin and harvesting a hive are those
//! blocks' own item legs.

use bbb_protocol::packets::BlockPos;

use super::ItemUseOnContext;
use crate::block_behavior::growing_plant;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::block_queries::int_property;

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    let pos: BlockPos = context.pos();
    let state = world.block_state_or_air(context.dimension, pos);
    if !growing_plant::is_head(state)
        || int_property(state, "age").is_none_or(|age| age >= growing_plant::MAX_AGE)
    {
        return false;
    }
    let Some(new_state) = with_property(state, "age", &growing_plant::MAX_AGE.to_string()) else {
        return false;
    };
    // `ShearsItem.java:69-71`: trigger before `setBlockAndUpdate` so the location sees the old block.
    world.trigger_item_used_on_location(
        context.connection_id,
        "minecraft:item_used_on_block",
        pos,
        context.stack,
        events,
    );
    world.play_block_sound_as(
        context.dimension,
        pos,
        "minecraft:block.growing_plant.crop",
        (1.0, 1.0),
        Some(context.connection_id),
        events,
    );
    world.set_block(context.dimension, pos, new_state, SET_BLOCK_UPDATE_ALL);
    let source = world.player_entity_id(context);
    world
        .level_mut_by_id(&context.dimension)
        .event_sink
        .game_event_with_source(
            VanillaGameEvent::BLOCK_CHANGE,
            pos,
            Some(source),
            Some(new_state),
        );
    world.hurt_and_break_in_hand(context.connection_id, context.hand(), 1, events);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionId;
    use crate::tests::{join_loaded, mailbox};
    use bbb_protocol::block_state_types::{
        vanilla_block_default_state_id, vanilla_block_state_with_property,
    };
    use bbb_protocol::block_types::VANILLA_BLOCK_WEEPING_VINES_ID;
    use bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID;
    use bbb_protocol::packets::{
        BlockHitResult, Direction, GameType, InteractionHand, ItemStackSummary, UseItemOn,
    };
    use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};
    use bbb_server_types::item_stack::RuntimeItemStack;
    use bbb_server_types::save::Java26BuiltinDimension;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn shearing_growing_plant_triggers_item_used_on_block() {
        let root = std::env::temp_dir().join(format!("bbb-shears-crit-{}", uuid::Uuid::new_v4()));
        let pack = root.join("pack");
        let adv_dir = pack.join("data/test/advancement");
        std::fs::create_dir_all(&adv_dir).unwrap();
        std::fs::write(
            pack.join("pack.mcmeta"),
            r#"{"pack":{"description":"shears test","min_format":101,"max_format":101}}"#,
        )
        .unwrap();
        std::fs::write(
            adv_dir.join("sheared_vines.json"),
            serde_json::to_vec(&json!({
                "criteria": {
                    "used": {
                        "trigger": "minecraft:item_used_on_block",
                        "conditions": {
                            "location": [
                                {"condition": "minecraft:location_check", "predicate": {"block": {"blocks": "minecraft:weeping_vines", "state": {"age": "0"}}}},
                                {"condition": "minecraft:match_tool", "predicate": {"items": "minecraft:shears"}}
                            ]
                        }
                    }
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let startup = Arc::new(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: Some(root.clone()),
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .unwrap(),
        );
        std::fs::remove_dir_all(&root).unwrap();

        let (sender, receiver) = mailbox();
        let mut config = crate::tests::datapack_config();
        config.game_type = GameType::Survival;
        config.datapacks = Some(startup);
        let mut world = ServerWorld::new(config).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        world.tick(receiver.drain());

        let player_pos = world.levels[ConnectionId(1)].position();
        let pos = BlockPos {
            x: player_pos.x.floor() as i32 + 1,
            y: player_pos.y.floor() as i32,
            z: player_pos.z.floor() as i32,
        };
        let default_vines = vanilla_block_default_state_id(VANILLA_BLOCK_WEEPING_VINES_ID).unwrap();
        let young_vines = vanilla_block_state_with_property(default_vines, "age", "0").unwrap();
        world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            young_vines,
            SET_BLOCK_UPDATE_ALL,
        );
        let shears = RuntimeItemStack::from_summary(ItemStackSummary {
            item_id: Some(VANILLA_ITEM_SHEARS_ID),
            count: 1,
            component_patch: Default::default(),
        });
        *world.levels[ConnectionId(1)].inventory.selected_item_mut() = shears.clone();
        let update = UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos,
                direction: Direction::North,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: false,
                world_border_hit: false,
            },
            sequence: 1,
        };
        let context = ItemUseOnContext {
            connection_id: ConnectionId(1),
            dimension: world.levels[ConnectionId(1)].dimension,
            update,
            stack: shears.summary(),
        };
        let mut events = Vec::new();
        assert!(use_on(&mut world, &context, &mut events));
        assert_eq!(
            int_property(
                world.block_state_or_air(Java26BuiltinDimension::Overworld, pos),
                "age"
            ),
            Some(growing_plant::MAX_AGE)
        );
        let index = world
            .cold
            .advancement_tree
            .index_of("test:sheared_vines")
            .expect("test advancement loaded");
        assert!(
            world.levels[ConnectionId(1)]
                .cold
                .advancements
                .progress(index)
                .is_some_and(|progress| progress.is_done()),
            "shearing growing plant must fire minecraft:item_used_on_block with the old block state"
        );
    }
}
