use super::{HeldItem, ItemUseContext, ItemUseOutcome, ItemUseResult};
use crate::world::{ServerWorld, WorldEvent};
use bbb_protocol::packets::{SoundEventHolder, SoundSource};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

// EmptyMapItem.java:17-35: consume, stat, sound, then replace the empty hand or add/drop the map.
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseOutcome {
    let resolver = world.item_stack_resolver();
    let player = world
        .levels
        .player_mut(context.connection_id)
        .expect("connected player");
    let position = player.position();
    let creative = player.abilities.instabuild;
    player.inventory.consume_in_hand(context.hand, 1, creative);
    if let Some(item_id) = context.item.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    world.play_player_sound(
        context.connection_id,
        None,
        SoundEventHolder::Reference {
            registry_id: vanilla_sound_event_registry_id(
                "minecraft:ui.cartography_table.take_result",
            )
            .expect("Vanilla cartography sound"),
        },
        SoundSource::Players,
        position,
        1.0,
        1.0,
        events,
    );
    let result = (|| -> bbb_server_types::Result<ItemUseOutcome> {
        let map = world.create_map(
            context.dimension,
            position.x.floor() as i32,
            position.z.floor() as i32,
            0,
            true,
            false,
        )?;
        let inventory = &mut world.levels[context.connection_id].inventory;
        if inventory.item_in_hand_mut(context.hand).is_empty() {
            return Ok(ItemUseOutcome {
                result: ItemUseResult::Success,
                transformed: Some(HeldItem::Replaced(map)),
            });
        }
        let mut copy = map.clone();
        if !inventory.add_anywhere(&resolver, &mut copy, creative)? {
            world.drop_player_stack(context.connection_id, &map, false);
        }
        Ok(ItemUseResult::Success.into())
    })();
    result.unwrap_or_else(|error| {
        tracing::warn!("empty map use: {error}");
        ItemUseResult::Fail.into()
    })
}
