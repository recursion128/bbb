use super::*;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};

fn summon_golem_over(feet: &str, head: &str) -> bool {
    let mut native = NativeLevel::new();
    for (y, block) in [
        (63, "minecraft:stone"),
        (64, feet),
        (65, head),
        (66, "minecraft:air"),
    ] {
        native.world.set_block(
            DIMENSION,
            BlockPos { x: 4, y, z: 4 },
            state(block),
            crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
        );
    }
    let dimension: LevelId = DIMENSION.into();
    native
        .world
        .try_spawn_mob(
            &dimension,
            crate::world::block_entity_removal::MobSpawnRequest {
                entity_type_id: VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
                start: BlockPos { x: 4, y: 64, z: 4 },
                attempts: 1,
                range_xz: 0,
                range_y: 1,
                strategy: SpawnStrategy::LegacyIronGolem,
                check_collisions: false,
                reason: crate::entity::SpawnReason::MobSummoned,
            },
        )
        .is_some()
}

#[test]
fn a_summoned_golem_stands_in_water_but_not_under_a_solid_block() {
    // IronGolem.java:305-323.
    assert!(summon_golem_over("minecraft:water", "minecraft:air"));
    assert!(!summon_golem_over("minecraft:air", "minecraft:stone"));
}

#[test]
fn breaking_a_furnace_drops_stored_xp_in_custom_datapack_dimension() {
    use bbb_protocol::item_types::{VANILLA_ITEM_COAL_ID, VANILLA_ITEM_RAW_GOLD_ID};
    use bbb_protocol::packets::ItemStackSummary;
    use bbb_server_types::save::{Java26BuiltinDimension, Java26LevelKey};

    let mut candidate = crate::tests::restore_candidate(0, None, Vec::new());
    let custom = Java26LevelKey::parse("example:moon").unwrap();
    let overworld = Java26LevelKey::from(Java26BuiltinDimension::Overworld);
    let stem = candidate.world_gen_settings.dimensions[&overworld].clone();
    candidate
        .world_gen_settings
        .dimensions
        .insert(custom.clone(), stem);
    candidate.raids.insert(custom.clone(), Default::default());
    candidate.world_borders.insert(
        custom.clone(),
        bbb_server_persistence::Java26WorldBorderSettings::default(),
    );
    let mut world = ServerWorld::restore(crate::tests::datapack_config(), &candidate).unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "execute in example:moon run forceload add 0 0",
    );
    let level_id = world.require_level_id(&custom);
    let at = BlockPos { x: 4, y: 64, z: 4 };
    assert!(world.set_block(
        level_id,
        at,
        state("minecraft:furnace"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let raw_gold = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_RAW_GOLD_ID),
        count: 1,
        component_patch: Default::default(),
    };
    let coal = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_COAL_ID),
        count: 1,
        component_patch: Default::default(),
    };
    world
        .level_mut_by_id(&level_id)
        .chunks
        .block_entities_mut()
        .get_mut(at)
        .and_then(crate::block_entity::BlockEntity::furnace_mut)
        .expect("furnace block entity")
        .set_items(
            crate::inventory::test_item_resolver(),
            &[
                crate::tests::native_stack(raw_gold),
                crate::tests::native_stack(coal),
                crate::tests::native_stack(ItemStackSummary::empty()),
            ],
            200,
        )
        .expect("furnace items");
    for _ in 0..200 {
        world.tick(receiver.drain());
    }
    assert!(world.set_block(
        level_id,
        at,
        state("minecraft:air"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());

    let level = world.level_by_id(&level_id);
    let orbs: Vec<(i32, Vec3d)> = level
        .entities
        .ids()
        .into_iter()
        .filter_map(|id| {
            let entity = level.entities.get(id)?;
            let EntityData::ExperienceOrb(orb) = entity.data() else {
                return None;
            };
            Some((orb.value(), entity.position()))
        })
        .collect();
    assert_eq!(
        orbs,
        vec![(
            1,
            Vec3d {
                x: 4.5,
                y: 64.5,
                z: 4.5,
            }
        )]
    );
}
