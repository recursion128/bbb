use super::*;
use crate::entity::{EntityData, EntityTickOutcome, FallingBlockData};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{
    vanilla_concrete_block_type_id_for_powder, VanillaGetStateForPlacementOverride,
    VANILLA_BLOCK_TYPES,
};
use bbb_server_types::directions::relative;

const LAND: BlockPos = BlockPos { x: 8, y: 64, z: 8 };
const AT: Vec3d = Vec3d {
    x: 8.5,
    y: 64.0,
    z: 8.5,
};

fn state(name: &str) -> i32 {
    let entry = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
    vanilla_block_default_state_id(entry.id).unwrap()
}

fn all_powders() -> impl Iterator<Item = &'static bbb_protocol::block_types::BlockTypeCatalogEntry>
{
    VANILLA_BLOCK_TYPES.iter().filter(|entry| {
        entry.get_state_for_placement_override
            == Some(VanillaGetStateForPlacementOverride::ConcretePowderBlock)
    })
}

fn setup_floor(level: &mut LevelCtx<'_, '_>) {
    level
        .write_block(BlockPos { y: 63, ..LAND }, state("minecraft:gold_block"))
        .unwrap();
}

#[test]
fn concrete_land_actual_falling_tick_owns_all_colors_dry_source_side_and_sturdy_cases() {
    let powders: Vec<_> = all_powders().collect();
    assert_eq!(powders.len(), 16);
    // Every color hardens in still water; the other landings run for one color.
    let cases = powders
        .iter()
        .map(|entry| (entry, 1))
        .chain([0, 2, 3].map(|mode| (&powders[0], mode)));
    with_config(crate::tests::datapack_config(), |level| {
        for (entry, mode) in cases {
            setup_floor(level);
            for pos in [LAND, relative(LAND, Direction::North)] {
                level.write_block(pos, state("minecraft:air")).unwrap();
            }
            let powder = vanilla_block_default_state_id(entry.id).unwrap();
            let concrete = vanilla_block_default_state_id(
                vanilla_concrete_block_type_id_for_powder(entry.id).unwrap(),
            )
            .unwrap();
            if mode == 1 {
                level.write_block(LAND, state("minecraft:water")).unwrap();
            }
            if mode == 2 {
                let flowing =
                    crate::fluid::with_property(state("minecraft:water"), "level", "5").unwrap();
                level
                    .write_block(relative(LAND, Direction::North), flowing)
                    .unwrap();
            }
            if mode == 3 {
                let barrier =
                    crate::fluid::with_property(state("minecraft:barrier"), "waterlogged", "true")
                        .unwrap();
                level
                    .write_block(relative(LAND, Direction::North), barrier)
                    .unwrap();
            }
            let mut store = batch_store();
            let id = store.spawn(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
                AT,
                EntityData::FallingBlock(FallingBlockData::new(powder, LAND)),
            );
            assert!(matches!(
                crate::entity::tick_one_entity(&mut store, level, id),
                EntityTickOutcome::Complete
            ));
            assert!(store.get(id).is_none(), "{} mode {mode}", entry.resource_id);
            assert_eq!(
                level.block_state(LAND),
                if mode == 1 || mode == 2 {
                    concrete
                } else {
                    powder
                },
                "{} mode {mode}",
                entry.resource_id
            );
        }
    });
}

#[test]
fn concrete_land_sturdy_shape_uses_landing_position_instead_of_neighbor_block_entity() {
    with_config(crate::tests::datapack_config(), |level| {
        setup_floor(level);
        let powder = state("minecraft:white_concrete_powder");
        assert!(LevelCtx::set_block(
            level,
            LAND,
            powder,
            2,
            crate::neighbor_updater::UPDATE_LIMIT
        ));
        let neighbor = relative(LAND, Direction::North);
        let shulker = state("minecraft:shulker_box");
        for direction in bbb_server_types::block_queries::DIRECTION_VALUES {
            let pos = relative(LAND, direction);
            let gold = state("minecraft:gold_block");
            if level.block_state(pos) != gold {
                assert!(LevelCtx::set_block(
                    level,
                    pos,
                    gold,
                    2,
                    crate::neighbor_updater::UPDATE_LIMIT
                ));
            }
        }
        assert!(LevelCtx::set_block(
            level,
            neighbor,
            shulker,
            2,
            crate::neighbor_updater::UPDATE_LIMIT
        ));
        assert!(level.chunks.block_entities().get(neighbor).is_some());
        assert!(level.chunks.block_entities().get(LAND).is_none());
        let tags = crate::registry_tags::RegistryTagSnapshot::new([(
            "minecraft:fluid".to_owned(),
            "minecraft:water".to_owned(),
            vec!["minecraft:empty".to_owned()],
        )]);
        level
            .chunks
            .install_registry_tags(std::sync::Arc::new(tags));
        assert_eq!(
            LevelCtx::concrete_powder_on_land(
                level,
                crate::entity::level::EntityBlockContext::new(None, None),
                powder,
                LAND,
                state("minecraft:water")
            ),
            Some(())
        );
        assert_eq!(level.block_state(LAND), powder);
    });
}

#[test]
fn concrete_land_uses_the_installed_water_fluid_tag() {
    with_config(crate::tests::datapack_config(), |level| {
        setup_floor(level);
        let powder = state("minecraft:white_concrete_powder");
        let concrete = state("minecraft:white_concrete");
        for (members, replaced, solidifies) in [
            (vec!["minecraft:lava"], state("minecraft:lava"), true),
            (vec!["minecraft:lava"], state("minecraft:water"), false),
            (vec![], state("minecraft:water"), false),
        ] {
            let _ = LevelCtx::set_block(
                level,
                LAND,
                powder,
                2 | 16,
                crate::neighbor_updater::UPDATE_LIMIT,
            );
            assert_eq!(level.block_state(LAND), powder);
            let tags = crate::registry_tags::RegistryTagSnapshot::new([(
                "minecraft:fluid".to_owned(),
                "minecraft:water".to_owned(),
                members.into_iter().map(str::to_owned).collect(),
            )]);
            level
                .chunks
                .install_registry_tags(std::sync::Arc::new(tags));
            assert_eq!(
                LevelCtx::concrete_powder_on_land(
                    level,
                    crate::entity::level::EntityBlockContext::new(None, None),
                    powder,
                    LAND,
                    replaced,
                ),
                Some(())
            );
            assert_eq!(
                level.block_state(LAND),
                if solidifies { concrete } else { powder }
            );
        }
    });
}
