use super::*;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
    VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use std::sync::OnceLock;

fn key() -> Java26LevelKey {
    Java26BuiltinDimension::Overworld.into()
}

/// The block a wave spawns onto, with stone under it.
fn floor() -> BlockPos {
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
        80,
        1,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
        )
        .unwrap(),
    );
    level.chunks.flush_live_relights().unwrap();
    level.sky_darkening = Some(0);
    level.random.reseed(0);
    world
}

fn request(entity_type_id: i32) -> RaidSpawn {
    RaidSpawn {
        entity_type_id,
        position: floor(),
        raid_id: 3,
        wave: 1,
        leader: false,
        enchant_odds: 0.0,
        enchant_provider: None,
        rides: None,
    }
}

fn spawned(world: &mut ServerWorld, spawn: RaidSpawn) -> Uuid {
    match world
        .spawn_one_raider(&world.require_level_id(&key()), spawn)
        .expect("a raid level")
    {
        RaidSpawnOutcome::Spawned { uuid, .. } => uuid,
        RaidSpawnOutcome::Refused(detail) => panic!("{detail}"),
    }
}

fn entity_id(world: &ServerWorld, uuid: Uuid) -> i32 {
    world
        .level_by_id(&world.require_level_id(&key()))
        .entities
        .entity_id_for_uuid(uuid)
        .expect("the published raider")
}

/// `Raid.joinRaid` (`Raid.java:578-584`): every wave type reaches the entity
/// store, half a block into its position and one above it, standing on the
/// block the search picked and holding the raid it joined.
#[test]
fn every_wave_type_reaches_the_entity_store_bound_to_its_raid() {
    let mut world = world();
    for entity_type_id in [
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        VANILLA_ENTITY_TYPE_WITCH_ID,
        VANILLA_ENTITY_TYPE_RAVAGER_ID,
    ] {
        let uuid = spawned(&mut world, request(entity_type_id));
        let id = entity_id(&world, uuid);
        let level = world.level_by_id(&world.require_level_id(&key()));
        let entity = level.entities.get(id).expect("the published raider");
        assert_eq!(entity.entity_type_id(), entity_type_id);
        assert_eq!(
            entity.position(),
            Vec3d {
                x: 1.5,
                y: 81.0,
                z: 1.5
            }
        );
        assert!(entity.on_ground());
        let NbtSummaryValue::Compound(data) = level.entities.command_data(id).unwrap() else {
            panic!("a raider is a compound");
        };
        let raid_id = data
            .iter()
            .find(|entry| entry.name == "RaidId")
            .map(|entry| entry.value.clone());
        assert_eq!(raid_id, Some(NbtSummaryValue::Int(3)));
    }
}

/// `Raid.java:536-540` and `PatrollingMonster.java:75-78`: the leader the wave
/// picked wears the ominous banner the EVENT finalize equips it with.
#[test]
fn the_leader_of_a_wave_wears_the_ominous_banner() {
    let mut world = world();
    let plain = spawned(&mut world, request(VANILLA_ENTITY_TYPE_PILLAGER_ID));
    let leader = spawned(
        &mut world,
        RaidSpawn {
            leader: true,
            ..request(VANILLA_ENTITY_TYPE_PILLAGER_ID)
        },
    );
    let level = world.level_by_id(&world.require_level_id(&key()));
    let head = |uuid: Uuid| {
        level
            .entities
            .get(entity_id(&world, uuid))
            .and_then(|entity| entity.living())
            .and_then(|living| living.item_by_slot(EquipmentSlot::Head).cloned())
    };
    assert!(head(plain).is_none());
    let banner = head(leader).expect("the leader's banner");
    assert_eq!(banner.item_resource_id(), Some("minecraft:white_banner"));
    assert!(crate::entity::patrol_spawn_state(
        level.entities.get(entity_id(&world, leader)).unwrap()
    )
    .is_some_and(|(_, leader, _)| leader));
}

/// `Vindicator.java:138-140`: the axe a vindicator with no raid takes from its
/// own finalize is the raid's to give, so `applyRaidBuffs` hands it over
/// instead (`Vindicator.java:168-180`).
#[test]
fn a_raid_vindicator_takes_its_axe_from_the_raid_buffs() {
    let mut world = world();
    let uuid = spawned(&mut world, request(VANILLA_ENTITY_TYPE_VINDICATOR_ID));
    let level = world.level_by_id(&world.require_level_id(&key()));
    let main_hand = level
        .entities
        .get(entity_id(&world, uuid))
        .and_then(|entity| entity.living())
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand).cloned())
        .expect("the vindicator's axe");
    assert_eq!(main_hand.item_resource_id(), Some("minecraft:iron_axe"));
}

/// `Raid.java:556-560`: the rider a ravager carries is published already
/// mounted on it.
#[test]
fn the_rider_a_ravager_carries_is_published_onto_it() {
    let mut world = world();
    let ravager = spawned(&mut world, request(VANILLA_ENTITY_TYPE_RAVAGER_ID));
    let rider = spawned(
        &mut world,
        RaidSpawn {
            rides: Some(ravager),
            ..request(VANILLA_ENTITY_TYPE_EVOKER_ID)
        },
    );
    let level = world.level_by_id(&world.require_level_id(&key()));
    let ravager_id = entity_id(&world, ravager);
    let rider_id = entity_id(&world, rider);
    assert_eq!(level.entities.vehicle(rider_id), Some(ravager_id));
    assert_eq!(level.entities.passengers(ravager_id), [rider_id]);
}

/// `Vindicator.java:168-180`: a vindicator whose enchant roll succeeds joins
/// the wave holding an axe the `raid/vindicator` provider enchanted.
#[test]
fn a_raider_whose_enchant_roll_succeeds_spawns_with_an_enchanted_weapon() {
    let mut world = world();
    let uuid = spawned(
        &mut world,
        RaidSpawn {
            enchant_odds: 1.0,
            enchant_provider: Some("minecraft:raid/vindicator"),
            ..request(VANILLA_ENTITY_TYPE_VINDICATOR_ID)
        },
    );
    let level = world.level_by_id(&world.require_level_id(&key()));
    let axe = level
        .entities
        .get(entity_id(&world, uuid))
        .and_then(|entity| entity.living())
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand).cloned())
        .expect("the vindicator's axe");
    assert_eq!(axe.item_resource_id(), Some("minecraft:iron_axe"));
    let plain = crate::entity::item::plain_stack("minecraft:iron_axe", 1).unwrap();
    assert!(!axe.same_item_same_components(&plain));
}

#[test]
fn raids_tick_and_spawn_raiders_in_custom_datapack_dimension() {
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
    world.admin.set_difficulty(Difficulty::Hard);
    let level_id = world.require_level_id(&custom);
    let level = world.level_mut_by_id(&level_id);
    level.chunks.load_column(0, 0);
    level.chunks.set_block_state(
        1,
        80,
        1,
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
        )
        .unwrap(),
    );
    level.chunks.flush_live_relights().unwrap();
    level.sky_darkening = Some(0);
    level.random.reseed(0);

    let mut events = Vec::new();
    world.tick_level_raids(&level_id, &mut events);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, WorldEvent::ConsoleMessage(_))),
        "custom dimension raid tick should not fail with unknown dimension: {events:?}"
    );

    let spawned = world
        .spawn_one_raider(&level_id, request(VANILLA_ENTITY_TYPE_PILLAGER_ID))
        .expect("custom dimension raid spawn");
    let RaidSpawnOutcome::Spawned { uuid, .. } = spawned else {
        panic!("expected raider spawn in custom dimension");
    };
    assert!(world
        .level_by_id(&level_id)
        .entities
        .entity_id_for_uuid(uuid)
        .is_some());
}
