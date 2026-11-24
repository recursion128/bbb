use super::*;
use crate::world::ServerWorld;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_server_types::save::Java26BuiltinDimension;

mod border_collision;
mod climbable_history;
mod falling_block_data;

fn with_level(test: impl FnOnce(&mut LevelCtx<'_, '_>)) {
    with_config(Default::default(), test);
}

fn with_config(config: crate::world::ServerWorldConfig, test: impl FnOnce(&mut LevelCtx<'_, '_>)) {
    with_config_and_players(config, &[], test);
}

fn with_config_and_players(
    config: crate::world::ServerWorldConfig,
    players: &[NearbyPlayer],
    test: impl FnOnce(&mut LevelCtx<'_, '_>),
) {
    with_world_and_players(ServerWorld::new(config).unwrap(), players, test);
}

fn with_world_and_players(
    world: ServerWorld,
    players: &[NearbyPlayer],
    test: impl FnOnce(&mut LevelCtx<'_, '_>),
) {
    with_world_frame(world, players, false, test);
}

/// The overworld frame, with its entity store lent to the level when
/// `lend_store` is set, as block ticks have it.
fn with_world_frame(
    mut world: ServerWorld,
    players: &[NearbyPlayer],
    lend_store: bool,
    test: impl FnOnce(&mut LevelCtx<'_, '_>),
) {
    let block_interaction_policy =
        world.block_interaction_policy(&Java26BuiltinDimension::Overworld.into());
    let (server, drop_tables, loot_level_facts, sculk_warning, structure_services, originals) =
        world.level_drop_tables_and_sculk_warning(&Java26BuiltinDimension::Overworld.into());
    server
        .simulation_levels
        .rebuild([((0, 0), CHUNK_ENTITY_TICKING_LEVEL)]);
    let mut leftover = PendingEntitySpawns::default();
    let (mut level, rest) = LevelCtx::new(
        server,
        LevelFrame {
            block_interaction_policy: &block_interaction_policy,
            full_preparation: FullChunkPreparationInputs {
                originals,
                entity_nbt: &structure_services.entity_nbt,
            },
            game_time: 10,
            game_rules: SimulationGameRules::vanilla_defaults(),
            feature_flags: SimulationFeatureFlags::default(),
            drop_tables,
            loot_level_facts,
            structure_services: Some(&structure_services),
            players,
            item_spawns: &mut leftover,
        },
    );
    level.sculk_warning = Some(sculk_warning);
    if lend_store {
        test(&mut level.with_structure_entities(rest.entities, None));
    } else {
        test(&mut level);
    }
}

fn block_tick_triggers_at(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Vec<i64> {
    level
        .chunks
        .tick_container::<BlockTickType>(chunk_of_block(pos))
        .map_or_else(Vec::new, |container| {
            container
                .iter()
                .filter(|tick| tick.pos == pos)
                .map(|tick| tick.trigger_tick)
                .collect()
        })
}

fn batch_store() -> EntityStore {
    EntityStore::new(17, crate::entity::EntityIdCounter::new())
}

mod item_fire;

mod item_orb_hurt;

mod simple_nonliving_hurt;

mod shulker_environment;

mod hanging_drops;

mod gift_loot;

mod hanging_piston;

mod ground_fall;

mod ground_eggs;

mod ground_steps;

mod ground_sensors;

mod stored_sensors;

mod ground_shriekers;

mod falling_contact;

mod concrete_land;

#[cfg(test)]
mod falling_source_clip;

mod falling_dynamic;

mod stored_context;

mod nonliving_context;

mod living_travel;

mod living_fall_fluid;
mod piston_cursor;

mod living_fall_particles;

mod push_climbable_history;
mod stored_fall_resetting_ray;

mod crop_contacts;
