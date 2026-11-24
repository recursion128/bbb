//! `HangingEntityItem.useOn` (`HangingEntityItem.java:34-76`) and
//! `ItemFrameItem.mayPlace` (`ItemFrameItem.java:15-17`): a painting, item
//! frame or glow item frame hung on the clicked face.

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    VANILLA_ENTITY_TYPE_PAINTING_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_GLOW_ITEM_FRAME_ID, VANILLA_ITEM_ITEM_FRAME_ID, VANILLA_ITEM_PAINTING_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, SoundSource};
use bbb_server_types::directions::relative;

use super::ItemUseOnContext;
use crate::entity::hanging::{
    placement_candidate, spawn_candidate, survives, PaintingVariant, PAINTING_VARIANT_REGISTRY,
};
use crate::entity::store::PendingEntitySound;
use crate::entity::Entity;
use crate::level_event::VanillaGameEvent;
use crate::world::block_use::spawn_egg::apply_default_stack_config;
use crate::world::{ServerWorld, WorldEvent};

const PLACEABLE_TAG: &str = "minecraft:placeable";

pub(super) fn use_on(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    _events: &mut Vec<WorldEvent>,
) -> bool {
    let face = context.update.hit.direction;
    let pos = relative(context.pos(), face);
    let vertical = matches!(face, Direction::Up | Direction::Down);
    let (entity_type_id, place_sound) = match context.stack.item_id {
        Some(VANILLA_ITEM_PAINTING_ID) => (
            VANILLA_ENTITY_TYPE_PAINTING_ID,
            "minecraft:entity.painting.place",
        ),
        Some(VANILLA_ITEM_ITEM_FRAME_ID) => (
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            "minecraft:entity.item_frame.place",
        ),
        Some(VANILLA_ITEM_GLOW_ITEM_FRAME_ID) => (
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
            "minecraft:entity.glow_item_frame.place",
        ),
        _ => return false,
    };
    // `:40-42`; the `mayUseItemAt` half is the caller's.
    let may_place = if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        !vertical
    } else {
        world.is_in_world_bounds_by_key(&context.dimension, pos)
    };
    if !may_place {
        return false;
    }
    let candidate = if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
        // `:47-50`: no placeable variant fits, a `CONSUME` that places nothing.
        let Some(candidate) = painting(world, context, pos, face) else {
            return true;
        };
        // `:63`: the stack's own `painting_variant` replaces the draw before
        // `survives` is asked.
        item_painting_variant(world, context)
            .and_then(|variant| {
                placement_candidate(VANILLA_ENTITY_TYPE_PAINTING_ID, pos, face, Some(variant))
            })
            .unwrap_or(candidate)
    } else {
        placement_candidate(entity_type_id, pos, face, None).expect("an item frame type")
    };
    let player_entity_id = world.player_entity_id(context);
    let Some(entity_id) = spawn_candidate(
        &mut world.level_mut_by_id(&context.dimension).entities,
        &candidate,
    ) else {
        return false;
    };
    // `:62-64`: the stack's config lands before `survives` is asked.
    apply_default_stack_config(
        world,
        Some(context.connection_id),
        context.dimension,
        entity_id,
        context.stack,
    );
    let placed = world.with_level_and_entities(context.dimension, |level, store| {
        let entity = store.take_fresh(entity_id)?;
        if !survives(&entity, store, level) {
            return None;
        }
        let position = entity.position();
        store.add_fresh(entity);
        Some(position)
    });
    // `:73-74`.
    let Some(position) = placed else {
        return true;
    };
    let level = world.level_mut_by_id(&context.dimension);
    // `:66-68`.
    level.entities.play_entity_sound(PendingEntitySound {
        position,
        resource_id: place_sound.to_owned(),
        source: SoundSource::Neutral,
        volume: 1.0,
        pitch: 1.0,
    });
    level.event_sink.game_event_with_source_at(
        VanillaGameEvent::ENTITY_PLACE,
        position,
        Some(player_entity_id),
        None,
    );
    world.shrink_used_stack(context);
    true
}

/// The stack's `painting_variant` (`Painting.java:77-90`).
fn item_painting_variant(
    world: &ServerWorld,
    context: &ItemUseOnContext<'_>,
) -> Option<PaintingVariant> {
    let id = context.stack.component_patch.painting_variant_id?;
    let items = world.item_stack_resolver();
    let resource_id = bbb_registry_codec::PersistentItemStackHolderResolver::holder_resource_id(
        &items,
        PAINTING_VARIANT_REGISTRY,
        id,
    )?;
    PaintingVariant::resolve(&items, &resource_id)
}

/// `Painting.create` (`Painting.java:92-119`): the largest `#placeable`
/// variants that fit, one drawn at random.
fn painting(
    world: &mut ServerWorld,
    context: &ItemUseOnContext<'_>,
    pos: BlockPos,
    face: Direction,
) -> Option<Entity> {
    let items = world.item_stack_resolver();
    let variants: Vec<PaintingVariant> = world
        .level_by_id(&context.dimension)
        .chunks
        .registry_tags()?
        .members(PAINTING_VARIANT_REGISTRY, PLACEABLE_TAG)
        .iter()
        .filter_map(|id| PaintingVariant::resolve(&items, id))
        .collect();
    let candidates: Vec<(Entity, i32)> = variants
        .into_iter()
        .filter_map(|variant| {
            let area = variant.area();
            placement_candidate(VANILLA_ENTITY_TYPE_PAINTING_ID, pos, face, Some(variant))
                .map(|entity| (entity, area))
        })
        .collect();
    let mut fitting = world.with_level_and_entities(context.dimension, |level, store| {
        candidates
            .into_iter()
            .filter(|(candidate, _)| survives(candidate, store, level))
            .collect::<Vec<_>>()
    });
    let largest = fitting.iter().map(|(_, area)| *area).max()?;
    fitting.retain(|(_, area)| *area == largest);
    let index = world.random.next_int(i32::try_from(fitting.len()).ok()?);
    Some(fitting.swap_remove(usize::try_from(index).ok()?).0)
}
