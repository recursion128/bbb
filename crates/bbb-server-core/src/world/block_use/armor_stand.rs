//! `ArmorStandItem.useOn` at the item leg of block interaction.

use crate::block_behavior::{can_replace_for_placement, BlockPlacementContext};
use crate::world::LevelId;
use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_ARMOR_STAND_ID;
use bbb_protocol::packets::{BlockPos, Direction, ItemStackSummary, SoundSource, UseItemOn, Vec3d};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;

use crate::entity::{armor_stand, PendingEntitySound};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::directions::relative;
use bbb_server_types::geometry::{collide_with_boxes, Aabb};
use bbb_server_types::stats::StatKey;

use super::ServerWorld;
use crate::world::entity_placement::{any_entity_in, block_collisions};

const PLACE_SOUND: &str = "minecraft:entity.armor_stand.place";

/// `BlockPlaceContext.getClickedPos` (`BlockPlaceContext.java:30`, `:52-57`)
/// for a held armor stand (`ArmorStandItem.java:34-35`).
pub(super) fn clicked_pos(
    world: &mut ServerWorld,
    dimension: LevelId,
    update: UseItemOn,
) -> BlockPos {
    let clicked_state = world.block_state_or_air(dimension, update.hit.pos);
    let context = BlockPlacementContext {
        clicked_face: update.hit.direction,
        replacing_clicked: true,
        ..BlockPlacementContext::from(update.hit.pos)
    };
    if can_replace_for_placement(VANILLA_ITEM_ARMOR_STAND_ID, clicked_state, context) == Some(true)
    {
        update.hit.pos
    } else {
        relative(update.hit.pos, update.hit.direction)
    }
}

/// `ArmorStandItem.useOn` (`ArmorStandItem.java:27-59`). `None` means the held
/// item was not an armor stand at all; `Some` is what `useOn` answered, as
/// `InteractionResult.consumesAction` (`InteractionResult.java:18-20`,
/// `:33-36`) reads it.
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    connection_id: super::ConnectionId,
    update: UseItemOn,
    stack: &ItemStackSummary,
) -> Option<bool> {
    if stack.item_id != Some(VANILLA_ITEM_ARMOR_STAND_ID) {
        return None;
    }
    if update.hit.direction == Direction::Down {
        return Some(false);
    }

    let dimension = world.player_level_id(connection_id);
    let target = clicked_pos(world, dimension, update);
    let base_position = Vec3d {
        x: f64::from(target.x) + 0.5,
        y: f64::from(target.y),
        z: f64::from(target.z) + 0.5,
    };
    let Some([width, height]) = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID)
    else {
        return Some(false);
    };
    let initial_box = Aabb::of_entity(base_position, width, height);
    if !placement_box_is_clear(world, dimension, initial_box) {
        return Some(false);
    }

    let y_offset = placement_y_offset(world, dimension, target, width, height);
    let final_position = Vec3d {
        y: f64::from(target.y) + y_offset,
        ..base_position
    };
    let player_y_rot = world.levels[connection_id].rotation().y_rot;
    let y_rot = ((super::wrap_degrees(player_y_rot - 180.0) + 22.5) / 45.0).floor() * 45.0;
    let source_entity_id = world.levels[connection_id].entity_id();
    let entity_id = {
        let level = world.level_mut_by_id(&dimension);
        let Some(entity_id) = armor_stand::spawn_placed(&mut level.entities, final_position, y_rot)
        else {
            return Some(false);
        };
        level.entities.play_entity_sound(PendingEntitySound {
            position: final_position,
            resource_id: PLACE_SOUND.to_owned(),
            source: SoundSource::Blocks,
            volume: 0.75,
            pitch: 0.8,
        });
        level.event_sink.game_event_with_source_at(
            VanillaGameEvent::ENTITY_PLACE,
            final_position,
            Some(source_entity_id),
            None,
        );
        entity_id
    };
    // `:41`: `EntityType.createDefaultStackConfig`.
    super::spawn_egg::apply_default_stack_config(
        world,
        Some(connection_id),
        dimension,
        entity_id,
        stack,
    );

    let player = world
        .levels
        .player_mut(connection_id)
        .expect("connection presence was checked");
    player
        .inventory
        .consume_in_hand(update.hand, 1, player.abilities.instabuild);
    player.award_stat_once(
        &mut world.scoreboard,
        StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, VANILLA_ITEM_ARMOR_STAND_ID),
    );
    Some(true)
}

fn placement_box_is_clear(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    box_to_test: Aabb,
) -> bool {
    let dimension = dimension.into();
    block_collisions(world, dimension, box_to_test).is_empty()
        && !any_entity_in(world, dimension, box_to_test, |_| true)
}

fn placement_y_offset(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    target: BlockPos,
    width: f32,
    height: f32,
) -> f64 {
    let dimension = dimension.into();
    // `EntityType.getYOffset` (`EntityType.java:1354-1362`).
    let entity_box = Aabb::of_entity(
        Vec3d {
            x: f64::from(target.x) + 0.5,
            y: f64::from(target.y) + 1.0,
            z: f64::from(target.z) + 0.5,
        },
        width,
        height,
    );
    let query = Aabb::of_block(target).expand_towards(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    let shapes: Vec<_> = block_collisions(world, dimension, query)
        .into_iter()
        .map(|(_, shape)| shape)
        .collect();
    let movement = collide_with_boxes(
        Vec3d {
            x: 0.0,
            y: -2.0,
            z: 0.0,
        },
        entity_box,
        &shapes,
    );
    1.0 + movement.y
}
