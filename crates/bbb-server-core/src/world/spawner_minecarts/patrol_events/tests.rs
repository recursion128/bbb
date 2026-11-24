use super::*;
use crate::entity::{patrol_spawn_state, EntityLifecycleEvent};
use bbb_protocol::equipment_slots::EquipmentSlot;
use std::sync::OnceLock;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}
fn position() -> BlockPos {
    BlockPos { x: 1, y: 80, z: 1 }
}
fn world() -> ServerWorld {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    let mut world =
        ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap();
    world.admin.set_difficulty(Difficulty::Hard);
    let level = world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap();
    level.chunks.load_column(0, 0);
    level.chunks.set_block_state(
        1,
        79,
        1,
        state(bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID),
    );
    level.chunks.flush_live_relights().unwrap();
    level.sky_darkening = Some(0);
    level.random.reseed(0);
    world
}
fn state(id: i32) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(id).unwrap()
}
fn set(world: &mut ServerWorld, pos: BlockPos, id: i32) {
    let chunks = &mut world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks;
    chunks.set_block_state(pos.x, pos.y, pos.z, state(id));
    chunks.flush_live_relights().unwrap();
}

#[test]
fn patrol_event_rejects_solid_peaceful_bright_block_light_and_missing_floor() {
    let mut world = world();
    set(
        &mut world,
        position(),
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    );
    assert_eq!(
        world.spawn_patrol_member(&world.require_level_id(&key()), position(), true),
        Ok(false)
    );
    set(
        &mut world,
        position(),
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    );
    world.admin.set_difficulty(Difficulty::Peaceful);
    assert_eq!(
        world.spawn_patrol_member(&world.require_level_id(&key()), position(), true),
        Ok(false)
    );
    world.admin.set_difficulty(Difficulty::Hard);
    set(
        &mut world,
        BlockPos { x: 2, y: 80, z: 1 },
        bbb_protocol::block_types::VANILLA_BLOCK_GLOWSTONE_ID,
    );
    assert!(
        world.levels[&world.require_level_id(&key())]
            .chunks
            .block_light_level(1, 80, 1)
            > 8
    );
    assert_eq!(
        world.spawn_patrol_member(&world.require_level_id(&key()), position(), true),
        Ok(false)
    );
    set(
        &mut world,
        BlockPos { x: 2, y: 80, z: 1 },
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    );
    set(
        &mut world,
        BlockPos { x: 1, y: 79, z: 1 },
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    );
    assert_eq!(
        world.spawn_patrol_member(&world.require_level_id(&key()), position(), true),
        Ok(false)
    );
    assert!(world.levels[&world.require_level_id(&key())]
        .entities
        .ids()
        .is_empty());
}

#[test]
fn patrol_event_leader_target_is_at_zero_and_finalized_state_is_published() {
    for leader in [false, true] {
        let mut world = world();
        assert_eq!(
            world.spawn_patrol_member(&world.require_level_id(&key()), position(), leader),
            Ok(true)
        );
        let entities = &mut world
            .levels
            .get_mut(&world.require_level_id(&key()))
            .unwrap()
            .entities;
        let ids = entities.ids();
        assert_eq!(ids.len(), 1);
        let entity = entities.get(ids[0]).unwrap();
        assert_eq!(
            entity.position(),
            Vec3d {
                x: 1.0,
                y: 80.0,
                z: 1.0
            }
        );
        assert_eq!(entity.old_position(), Vec3d::default());
        let (target, actual_leader, patrolling) = patrol_spawn_state(entity).unwrap();
        assert_eq!(actual_leader, leader);
        assert!(patrolling);
        if leader {
            let target = target.unwrap();
            assert_eq!(target.y, 0);
            assert!((-500..500).contains(&target.x));
            assert!((-500..500).contains(&target.z));
        } else {
            assert_eq!(target, None);
        }
        let living = entity.living().unwrap();
        assert!(living.item_by_slot(EquipmentSlot::MainHand).is_some());
        assert_eq!(living.item_by_slot(EquipmentSlot::Head).is_some(), leader);
        let NbtSummaryValue::Compound(fields) = entities.command_data(ids[0]).unwrap() else {
            panic!("compound");
        };
        for (name, value) in [
            ("PatrolLeader", leader),
            ("Patrolling", true),
            ("CanJoinRaid", true),
        ] {
            assert!(fields
                .iter()
                .any(|entry| entry.name == name
                    && entry.value == NbtSummaryValue::Byte(i8::from(value))));
        }
        let events = entities.drain_lifecycle_events();
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(event, EntityLifecycleEvent::Added { .. }))
                .count(),
            1
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            EntityLifecycleEvent::Sound { .. } | EntityLifecycleEvent::SoundFromEntity { .. }
        )));
    }
}

#[test]
fn patrol_event_specific_provider_resumes_into_patrol_finalization() {
    let mut world = world();
    let piercing = |world: &ServerWorld, id: i32| {
        let entity = world.levels[&world.require_level_id(&key())]
            .entities
            .get(id)
            .unwrap();
        let stack = entity
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .unwrap();
        let Some(NbtSummaryValue::Compound(values)) = stack
            .component_nbt(
                bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                &world.item_stack_resolver(),
            )
            .unwrap()
        else {
            return false;
        };
        values.iter().any(|entry| {
            entry.name == "minecraft:piercing" && entry.value == NbtSummaryValue::Int(1)
        })
    };
    // The pillager's own crossbow provider is a 1-in-300 roll; 20 000 spawns
    // all miss with odds of about e^-50.
    let id = (0..20_000)
        .find_map(|_| {
            assert_eq!(
                world.spawn_patrol_member(&world.require_level_id(&key()), position(), true),
                Ok(true)
            );
            let id = world.levels[&world.require_level_id(&key())]
                .entities
                .ids()
                .into_iter()
                .max()
                .unwrap();
            piercing(&world, id).then_some(id)
        })
        .expect("a pillager rolls the crossbow provider");
    let entity = world.levels[&world.require_level_id(&key())]
        .entities
        .get(id)
        .unwrap();
    let (_, leader, patrolling) = patrol_spawn_state(entity).unwrap();
    assert!(leader && patrolling);
    assert!(entity
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::Head)
        .is_some());
}
