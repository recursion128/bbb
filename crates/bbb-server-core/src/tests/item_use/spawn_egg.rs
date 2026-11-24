//! `SpawnEggItem.use` (`SpawnEggItem.java:110-136`): the liquid-source arm a
//! right click at nothing takes.

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, Vec3d};

use super::*;

/// The fixture floor's top is y = 63, so the session stands at y = 64 with its
/// eye at 65.62 and looks down +z.
const STANDING: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};

fn hold_spawn_eggs(world: &mut ServerWorld, name: &str, count: i32) {
    let item_id = bbb_protocol::item_types::vanilla_item_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("missing built-in item {name}"))
        .id;
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        0,
        ItemStackSummary {
            item_id: Some(item_id),
            count,
            ..ItemStackSummary::empty()
        },
    );
}

fn water(level: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &std::collections::BTreeMap::from([("level".to_string(), level.to_string())]),
    )
    .expect("the built-in water block state")
}

fn set_block(world: &mut ServerWorld, pos: BlockPos, block_state_id: i32) {
    assert!(
        world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            block_state_id,
            SET_BLOCK_UPDATE_ALL,
        ),
        "{pos:?} took the write"
    );
}

fn spawned_entity(world: &ServerWorld, entity_type_id: i32) -> Option<Vec3d> {
    world
        .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
        .into_iter()
        .find_map(|id| {
            let entity = world.stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
            (entity.entity_type_id() == entity_type_id).then(|| entity.position())
        })
}

fn item_used_count(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    resource_id: &str,
) -> i32 {
    let stats = play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    stats
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::AwardStats(update),
                ..
            } => Some(update),
            _ => None,
        })
        .expect("the stats answer is sent")
        .stats
        .iter()
        .find(|entry| {
            entry.stat_type_id == bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID
                && entry.value_id == item_id(resource_id)
        })
        .map_or(0, |entry| entry.amount)
}

fn survival(world: &mut ServerWorld) {
    world.set_player_mode_and_abilities_for_test(
        ConnectionId(1),
        GameType::Survival,
        PlayerAbilities::for_game_type(GameType::Survival),
    );
}

#[test]
fn a_spawn_egg_used_at_a_water_source_spawns_in_the_fluid_cell_and_awards_the_stat() {
    // `:112`: `getPlayerPOVHitResult(level, player, Fluid.SOURCE_ONLY)`;
    // `:124`: `spawnMob(..., pos, false, false)`, so the entity is placed in
    // the fluid's own cell rather than the one above it, and `EntityType.create`
    // skips its y-offset walk.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    survival(&mut world);
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 3);
    set_block(&mut world, BlockPos { x: 0, y: 65, z: 2 }, water("0"));

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert_eq!(
        spawned_entity(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID),
        Some(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 2.5,
        })
    );
    // `:101-103`: `itemStack.consume(1, user)` through `spawnMob`.
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
    // `:125-127`: `player.awardStat(Stats.ITEM_USED.get(this))`.
    assert_eq!(
        item_used_count(&sender, &receiver, &mut world, "minecraft:zombie_spawn_egg"),
        1
    );
}

#[test]
fn a_spawn_egg_used_at_a_block_that_is_not_a_liquid_passes() {
    // `:119-121`: the ray found a block, but only a `LiquidBlock` cell takes
    // the spawn.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    survival(&mut world);
    hold_spawn_eggs(&mut world, "minecraft:cow_spawn_egg", 2);
    set_block(
        &mut world,
        BlockPos { x: 0, y: 65, z: 2 },
        block_state_id("minecraft:stone"),
    );

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_COW_ID).is_none());
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
    assert_eq!(
        item_used_count(&sender, &receiver, &mut world, "minecraft:cow_spawn_egg"),
        0
    );
}

#[test]
fn a_spawn_egg_used_at_the_sky_finds_no_block() {
    // `:113-115`: `HitResult.Type.MISS` is a pass. The ray is
    // `blockInteractionRange` long, so a straight-up look from y = 65.62 ends
    // at y = 70.12 in air.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    survival(&mut world);
    hold_spawn_eggs(&mut world, "minecraft:cow_spawn_egg", 2);

    throw(&sender, &receiver, &mut world, 1, 0.0, -90.0, 1);

    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_COW_ID).is_none());
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
}

#[test]
fn a_spawn_egg_used_at_flowing_water_looks_through_to_the_source_behind_it() {
    // `ClipContext.Fluid.SOURCE_ONLY` (`ClipContext.java:35-40`) contributes a
    // shape only where `FluidState.isSource` holds, and a flowing water block's
    // own outline shape is empty (`LiquidBlock`), so the ray passes through the
    // flowing cell and stops on the source two cells further along.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick_rate_mut().set_frozen(true);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    survival(&mut world);
    hold_spawn_eggs(&mut world, "minecraft:cow_spawn_egg", 2);
    set_block(&mut world, BlockPos { x: 0, y: 65, z: 2 }, water("2"));
    set_block(&mut world, BlockPos { x: 0, y: 65, z: 4 }, water("0"));

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert_eq!(
        spawned_entity(&world, VANILLA_ENTITY_TYPE_COW_ID),
        Some(Vec3d {
            x: 0.5,
            y: 65.0,
            z: 4.5,
        })
    );
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 1);
}

#[test]
fn a_spawn_egg_used_past_the_interaction_range_finds_no_block() {
    // `Item.getPlayerPOVHitResult` (`Item.java:351`) scales the view vector by
    // `Player.blockInteractionRange`, which the fixture player's
    // `BLOCK_INTERACTION_RANGE` attribute answers as 4.5.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(datapack_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    survival(&mut world);
    hold_spawn_eggs(&mut world, "minecraft:cow_spawn_egg", 2);
    set_block(&mut world, BlockPos { x: 0, y: 65, z: 6 }, water("0"));

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_COW_ID).is_none());
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 2);
}

#[test]
fn a_monster_egg_used_at_a_water_source_refuses_in_peaceful() {
    // `:97-99` through `:124-129`: the refusal is `FAIL`, so nothing is
    // consumed and no stat is awarded.
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(ServerWorldConfig {
        difficulty: Difficulty::Peaceful,
        ..datapack_config()
    });
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_player_position_for_test(ConnectionId(1), STANDING);
    survival(&mut world);
    hold_spawn_eggs(&mut world, "minecraft:zombie_spawn_egg", 3);
    set_block(&mut world, BlockPos { x: 0, y: 65, z: 2 }, water("0"));

    throw(&sender, &receiver, &mut world, 1, 0.0, 0.0, 1);

    assert!(spawned_entity(&world, VANILLA_ENTITY_TYPE_ZOMBIE_ID).is_none());
    assert_eq!(world.publication().players[0].inventory.slots[0].count, 3);
    assert_eq!(
        item_used_count(&sender, &receiver, &mut world, "minecraft:zombie_spawn_egg"),
        0
    );
}
