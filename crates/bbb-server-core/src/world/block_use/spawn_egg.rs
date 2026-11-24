//! `SpawnEggItem.useOn` at the item leg of block interaction
//! (`SpawnEggItem.java:43-82`): the `Spawner` arm, which the trial spawner
//! reaches through its own `setEntityId`, and the spawn `spawnMob`
//! (`:83-105`) runs for every other block.

use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::{
    vanilla_entity_resource_id_for_type_id, vanilla_entity_type, vanilla_entity_type_dimensions,
};
use bbb_protocol::packets::{BlockPos, Direction, ItemStackSummary, SystemChat, UseItemOn, Vec3d};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::{component_translatable_nbt, ComponentStyle, RichTextComponent};
use bbb_protocol::{merge_command_nbt_compound, NbtSummaryEntry, NbtSummaryValue};

use crate::block_entity::BlockEntity;
use crate::connection::ConnectionId;
use crate::fluid::{with_property, SET_BLOCK_UPDATE_ALL};
use crate::level_event::VanillaGameEvent;
use bbb_server_types::directions::relative;
use bbb_server_types::geometry::{collide_axis, Aabb, Axis};
use bbb_server_types::stats::StatKey;

use super::{ServerWorld, WorldEvent};
use crate::world::movement_collision::{block_collision_boxes, block_collision_shape_checked};
use crate::world::LevelId;

/// `SpawnEggItem.useOn` (`SpawnEggItem.java:43-82`) over the spawner arm
/// (`:52-71`). True only when the egg was consumed: every refusal is
/// `InteractionResult.FAIL`, whose `consumesAction` is false
/// (`InteractionResult.java:18-20`).
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    update: UseItemOn,
    stack: &ItemStackSummary,
    events: &mut Vec<WorldEvent>,
) -> bool {
    if !crate::entity::held_is_spawn_egg(stack) {
        return false;
    }
    let dimension = world.player_level_id(connection_id);
    let pos = update.hit.pos;
    let is_spawner = world
        .get_block_entity(dimension, pos)
        .and_then(BlockEntity::spawner)
        .is_some();
    // `TrialSpawnerBlockEntity implements Spawner` (`TrialSpawnerBlockEntity.java:23`),
    // so `:52` reaches it too, through `overrideEntityToSpawn`
    // (`TrialSpawner.java:340-344`).
    let is_trial_spawner = world
        .get_block_entity(dimension, pos)
        .and_then(BlockEntity::trial_spawner)
        .is_some();
    if !is_spawner && !is_trial_spawner {
        // `:72-80`: an empty collision shape takes the clicked cell, and every
        // other block takes the cell the clicked face points at.
        let spawn_pos =
            if block_collision_shape_checked(&world.level_by_id(&dimension).chunks, pos, None)
                .is_some_and(|boxes| boxes.is_empty())
            {
                pos
            } else {
                relative(pos, update.hit.direction)
            };
        // `movedUp` is the clicked top face of a block whose cell was full.
        let moved_up = spawn_pos != pos && update.hit.direction == Direction::Up;
        let outcome = spawn_mob(
            world,
            connection_id,
            update.hand,
            stack,
            dimension,
            spawn_pos,
            true,
            moved_up,
        );
        // `ItemStack.useOn` (`ItemStack.java:367-369`) awards the stat for the
        // `SUCCESS` it answers, and `ServerPlayerGameMode.useItemOn`
        // (`:397-399`) triggers on a consuming result -- both whether or not
        // the level took the entity.
        if matches!(outcome, SpawnMobOutcome::Refused) {
            return false;
        }
        let player = world
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        if let Some(item_id) = stack.item_id {
            player.award_stat_once(
                &mut world.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        world.trigger_item_used_on_location(
            connection_id,
            "minecraft:item_used_on_block",
            pos,
            stack,
            events,
        );
        return true;
    }
    // `:53-56`.
    let Some(entity_type_id) = crate::entity::spawn_egg_entity_type_id(stack) else {
        return false;
    };
    // `:58-63`.
    if !world.game_rules.spawner_blocks_work {
        let message = RichTextComponent::try_from_component_nbt(
            component_translatable_nbt(
                "advMode.notEnabled.spawner",
                Vec::new(),
                &ComponentStyle::default(),
            )
            .expect("the spawner message is a validated component shape"),
        )
        .expect("the spawner message is a validated component shape");
        world.send_system_message(
            connection_id,
            SystemChat::from_component(message, false),
            events,
        );
        return false;
    }
    let Some(resource_id) = vanilla_entity_resource_id_for_type_id(entity_type_id).or_else(|| {
        world
            .cold
            .config
            .plugins
            .content_registry()
            .entity_by_id(entity_type_id)
            .map(|spec| spec.resource_id.clone())
    }) else {
        return false;
    };
    // `:65`: `spawnerHolder.setEntityId(type, level.getRandom())`.
    if is_trial_spawner {
        // `TrialSpawnerBlockEntity.setEntityId` (`:55-61`) then writes the
        // `INACTIVE` state the override asks for
        // (`TrialSpawner.overrideEntityToSpawn`'s `setState`, which goes
        // through `Level.setBlockAndUpdate`).
        let Some(trial_spawner) = world
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::trial_spawner_mut)
        else {
            return false;
        };
        trial_spawner.override_entity_to_spawn(&resource_id);
        let block_state_id = world.block_state_or_air(dimension, pos);
        if let Some(inactive) = with_property(block_state_id, "trial_spawner_state", "inactive") {
            world.set_block(dimension, pos, inactive, SET_BLOCK_UPDATE_ALL);
        }
    } else {
        // The spawner's `getOrCreateNextSpawnData` spends the level's draw
        // when `SpawnData` is unset, then `SpawnerBlockEntity.setEntityId`'s
        // own `setChanged`.
        let level = world.level_mut_by_id(&dimension);
        let Some(spawner) = level
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::spawner_mut)
        else {
            return false;
        };
        spawner.set_entity_id_drawing(&resource_id, &level.random);
    }
    // `:66-67`: `sendBlockUpdated(pos, state, state, 3)`, then
    // `gameEvent(player, BLOCK_CHANGE, pos)`.
    let player_entity_id = world.levels[connection_id].entity_id();
    let level = world.level_mut_by_id(&dimension);
    level.chunks.block_entities_mut().set_changed(pos);
    level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
    level.event_sink.game_event_with_source(
        VanillaGameEvent::BLOCK_CHANGE,
        pos,
        Some(player_entity_id),
        None,
    );
    // `:68` consumes one, which `ServerPlayerGameMode.useItemOn`'s infinite
    // materials branch restores (`ServerPlayerGameMode.java:389-393`);
    // `ItemStack.useOn` awards the stat for the `SUCCESS` it answers
    // (`ItemStack.java:365-368`).
    let has_infinite_materials = world.levels[connection_id].abilities.instabuild;
    let player = world
        .levels
        .player_mut(connection_id)
        .expect("connection presence was checked");
    player
        .inventory
        .consume_in_hand(update.hand, 1, has_infinite_materials);
    if let Some(item_id) = stack.item_id {
        player.award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    world.trigger_item_used_on_location(
        connection_id,
        "minecraft:item_used_on_block",
        pos,
        stack,
        events,
    );
    true
}

/// What `SpawnEggItem.spawnMob` (`SpawnEggItem.java:84-107`) answered, which
/// both `useOn` and `use` read as `InteractionResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::world) enum SpawnMobOutcome {
    /// `InteractionResult.SUCCESS` (`:104`): whether the level took the
    /// entity is not part of the answer, so the consume and the
    /// `ENTITY_PLACE` are the only things a failed spawn skips.
    Success,
    /// `InteractionResult.FAIL` (`:86`, `:90`): no entity type on the stack,
    /// or a monster the difficulty refuses.
    Refused,
}

/// `SpawnEggItem.spawnMob` (`SpawnEggItem.java:84-107`): the type the egg
/// carries, the peaceful refusal, then `EntityType.spawn` (`:1256-1273`) with
/// the stack's own config consumer, and the consume with its
/// `GameEvent.ENTITY_PLACE` when the level took the entity.
#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn spawn_mob(
    world: &mut ServerWorld,
    connection_id: ConnectionId,
    hand: bbb_protocol::packets::InteractionHand,
    stack: &ItemStackSummary,
    dimension: impl Into<LevelId>,
    spawn_pos: BlockPos,
    try_move_down: bool,
    moved_up: bool,
) -> SpawnMobOutcome {
    let dimension = dimension.into();
    // `:92-95`.
    let Some(entity_type_id) = crate::entity::spawn_egg_entity_type_id(stack) else {
        return SpawnMobOutcome::Refused;
    };
    // `:97-99`: a monster egg in peaceful refuses without consuming.
    let allowed_in_peaceful = vanilla_entity_type(entity_type_id)
        .map(|entry| entry.allowed_in_peaceful)
        .or_else(|| {
            world
                .cold
                .config
                .plugins
                .content_registry()
                .entity_by_id(entity_type_id)
                .map(|spec| spec.allowed_in_peaceful)
        })
        .unwrap_or(true);
    if !allowed_in_peaceful && world.admin.settings().difficulty == Difficulty::Peaceful {
        return SpawnMobOutcome::Refused;
    }
    let spawned = spawn_from_item(
        world,
        Some(connection_id),
        dimension,
        entity_type_id,
        spawn_pos,
        try_move_down,
        moved_up,
        stack,
        crate::entity::SpawnReason::SpawnItemUse,
    );
    if spawned.is_some() {
        // `:101-103`: `ItemStack.consume(1, user)` then
        // `gameEvent(user, ENTITY_PLACE, spawnPos)`.
        let player_entity_id = world.levels[connection_id].entity_id();
        let has_infinite_materials = world.levels[connection_id].abilities.instabuild;
        let player = world
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        player
            .inventory
            .consume_in_hand(hand, 1, has_infinite_materials);
        world
            .level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source(
                VanillaGameEvent::ENTITY_PLACE,
                spawn_pos,
                Some(player_entity_id),
                None,
            );
    }
    SpawnMobOutcome::Success
}

/// `EntityType.create` (`EntityType.java:1320-1351`) over
/// `EntityType.spawn` (`:1300-1312`): the y-offset walk, the yaw draw and the
/// fresh entity the level takes.
#[allow(clippy::too_many_arguments)]
pub(in crate::world) fn spawn_from_item(
    world: &mut ServerWorld,
    connection_id: Option<ConnectionId>,
    dimension: impl Into<LevelId>,
    entity_type_id: i32,
    spawn_pos: BlockPos,
    try_move_down: bool,
    moved_up: bool,
    stack: &ItemStackSummary,
    reason: crate::entity::SpawnReason,
) -> Option<i32> {
    let dimension = dimension.into();
    let [width, height] = vanilla_entity_type_dimensions(entity_type_id).or_else(|| {
        world
            .cold
            .config
            .plugins
            .content_registry()
            .entity_by_id(entity_type_id)
            .map(|spec| [spec.width, spec.height])
    })?;
    // `:1331-1332`: `setPos(x + 0.5, y + 1, z + 0.5)` before the walk.
    let initial = Vec3d {
        x: f64::from(spawn_pos.x) + 0.5,
        y: f64::from(spawn_pos.y) + 1.0,
        z: f64::from(spawn_pos.z) + 0.5,
    };
    let y_offset = if try_move_down {
        get_y_offset(
            world,
            &dimension,
            spawn_pos,
            moved_up,
            Aabb::of_entity(initial, width, height),
        )
    } else {
        0.0
    };
    // `:1340`: `Mth.wrapDegrees(level.getRandom().nextFloat() * 360.0F)`.
    let yaw = wrap_degrees(world.level_mut_by_id(&dimension).random.next_float() * 360.0);
    let position = Vec3d {
        x: initial.x,
        y: f64::from(spawn_pos.y) + y_offset,
        z: initial.z,
    };
    let general = world
        .level_mut_by_id(&dimension)
        .entities
        .spawn_general_living(entity_type_id, position);
    let entity_id = if let Some(entity_id) = general {
        // `:1341-1343`: the head follows the body on the spawn's own yaw.
        if let Some(entity) = world
            .level_mut_by_id(&dimension)
            .entities
            .get_mut(entity_id)
        {
            entity.set_rot(yaw, 0.0);
            if let Some(living) = entity.living_mut() {
                living.y_head_rot = yaw;
            }
        }
        world.finalize_added_mob(&dimension, entity_id, reason);
        entity_id
    } else {
        // `EntityType.create` takes any type, not only mobs (`:1320-1351`).
        world.spawn_finalized_entity(dimension, entity_type_id, position, reason, Some(yaw))?
    };
    apply_default_stack_config(world, connection_id, dimension, entity_id, stack);
    // `MobBucketItem.java:53` plays it only after loading the bucket tag.
    if reason != crate::entity::SpawnReason::Bucket {
        world.play_spawn_ambient(dimension, entity_id);
    }
    Some(entity_id)
}

impl ServerWorld {
    /// `mob.playAmbientSound()` right after `EntityType.spawn` adds a mob
    /// (`EntityType.java:1310-1312`).
    pub(in crate::world) fn play_spawn_ambient(&mut self, dimension: LevelId, entity_id: i32) {
        self.with_level_and_entities(dimension, |level, entities| {
            entities.play_spawn_ambient(entity_id, Some(level));
        });
    }
}

/// `EntityType.appendDefaultStackConfig` (`EntityType.java:1279-1294`): the
/// stack's components (`Entity.applyComponentsFromItemStack`, `Entity.java:3947-3953`),
/// then its `minecraft:entity_data` merged onto the fresh entity minus its `id`
/// (`updateCustomEntityTag`, `:1364-1379`).
pub(in crate::world) fn apply_default_stack_config(
    world: &mut ServerWorld,
    connection_id: Option<ConnectionId>,
    dimension: impl Into<LevelId>,
    entity_id: i32,
    stack: &ItemStackSummary,
) {
    let dimension = dimension.into();
    let publication = world.level_by_id(&dimension).entities.item_publication();
    let Some(entity) = world
        .level_mut_by_id(&dimension)
        .entities
        .get_mut(entity_id)
    else {
        return;
    };
    crate::entity::implicit_components::apply_components_from_stack(
        entity,
        stack,
        publication.resolver(),
    );
    let entity_type_id = entity.entity_type_id();
    let entity_resource_id = entity.data().entity_resource_id();
    let Some(NbtSummaryValue::Compound(entries)) = stack.component_patch.entity_data_tag.as_ref()
    else {
        return;
    };
    // `:1369`: the data must name the entity's own type.
    let typed_for = entries.iter().find(|entry| entry.name == "id");
    if let Some(NbtSummaryValue::String(id)) = typed_for.map(|entry| &entry.value) {
        if id != entity_resource_id {
            return;
        }
    }
    // `:1370-1371`: a type whose NBT only an operator may set needs one.
    let only_op_can_set_nbt =
        vanilla_entity_type(entity_type_id).is_some_and(|entry| entry.only_op_can_set_nbt);
    if only_op_can_set_nbt
        && !connection_id.is_some_and(|id| world.admin.is_operator(world.levels[id].profile.uuid))
    {
        return;
    }
    let entries: Vec<_> = entries
        .iter()
        .filter(|entry| entry.name != "id")
        .cloned()
        .collect();
    load_entity_nbt(world, dimension, entity_id, &entries);
}

/// `CompoundTag.merge` of `entries` onto the entity's own record, loaded back
/// (`EntityType.updateCustomEntityTag`, `EntityType.java:1364-1379`).
pub(in crate::world) fn load_entity_nbt(
    world: &mut ServerWorld,
    dimension: impl Into<LevelId>,
    entity_id: i32,
    entries: &[NbtSummaryEntry],
) {
    let dimension = dimension.into();
    if entries.is_empty() {
        return;
    }
    let input = world.entity_nbt_read_snapshot();
    let entities = &mut world.level_mut_by_id(&dimension).entities;
    let Ok(NbtSummaryValue::Compound(mut record)) = entities.command_data(entity_id) else {
        return;
    };
    merge_command_nbt_compound(&mut record, entries);
    let _ = entities.load_command_data_with_context(entity_id, &record, None, input.context());
    world.resolve_raider_raids_in_tree(&dimension, entity_id);
}

/// `EntityType.getYOffset` (`EntityType.java:1354-1362`): how far the entity
/// box falls from `y + 1` onto the collision shapes under the spawn cell.
fn get_y_offset(
    world: &ServerWorld,
    dimension: &LevelId,
    spawn_pos: BlockPos,
    moved_up: bool,
    entity_box: Aabb,
) -> f64 {
    let query = Aabb {
        min_x: f64::from(spawn_pos.x),
        min_y: f64::from(spawn_pos.y) - if moved_up { 1.0 } else { 0.0 },
        min_z: f64::from(spawn_pos.z),
        max_x: f64::from(spawn_pos.x) + 1.0,
        max_y: f64::from(spawn_pos.y) + 1.0,
        max_z: f64::from(spawn_pos.z) + 1.0,
    };
    let boxes = block_collision_boxes(&world.level_by_id(dimension).chunks, query);
    1.0 + collide_axis(
        Axis::Y,
        entity_box,
        &boxes,
        if moved_up { -2.0 } else { -1.0 },
    )
}

/// `Mth.wrapDegrees` (`Mth.java:39-46`).
fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}
