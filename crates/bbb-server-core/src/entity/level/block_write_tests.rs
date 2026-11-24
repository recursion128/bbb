use super::*;
use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use crate::level_event::VanillaGameEvent;

const POS: BlockPos = BlockPos { x: 2, y: 70, z: 4 };

#[test]
fn stored_fall_resetting_ray_reads_actual_blocks_and_installed_tags() {
    let stone = state("minecraft:stone");
    let air = state("minecraft:air");
    let from = Vec3d {
        x: f64::from(POS.x) + 0.5,
        y: f64::from(POS.y) + 0.5,
        z: f64::from(POS.z) + 0.5,
    };
    let to = Vec3d {
        x: from.x + 2.0,
        ..from
    };
    for custom_tags in [false, true] {
        let mut native = NativeLevel::new();
        let kind = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
        let (source, _) = native.world.spawn_stored_entity_for_test(
            DIMENSION,
            kind,
            from,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(kind).unwrap(),
            ),
        );
        if custom_tags {
            native.install_tags(RegistryTagSnapshot::new([(
                "minecraft:block".into(),
                "minecraft:fall_damage_resetting".into(),
                vec!["minecraft:stone".into()],
            )]));
        }
        native.with_level(|level| {
            assert_eq!(level.clip_fall_resetting(from, to), Some(false));
            level.set_block(POS, stone, 2, crate::neighbor_updater::UPDATE_LIMIT);
            level.game_event(VanillaGameEvent::BLOCK_CHANGE, POS, Some(source));
            assert_eq!(level.block_state(POS), stone);
            assert_eq!(level.clip_fall_resetting(from, to), Some(custom_tags));
            level.set_block(POS, air, 2, crate::neighbor_updater::UPDATE_LIMIT);
            assert_eq!(level.clip_fall_resetting(from, to), Some(false));
        });
        assert_eq!(native.world.block_state(DIMENSION, POS), Some(air));
        assert!(native
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(|event| {
                event.recorded.game_event == VanillaGameEvent::BLOCK_CHANGE
                    && event.recorded.source_entity_id == Some(source)
                    && event.position == from
            }));
    }
}
