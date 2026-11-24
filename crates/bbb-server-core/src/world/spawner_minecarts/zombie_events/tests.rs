use super::*;
use crate::entity::item::plain_stack;
use crate::entity::{MerchantItemCost, MerchantOffer, MerchantOffers};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIG_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use std::sync::OnceLock;

const OVERWORLD: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const ZOMBIE: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;

/// Columns -3..=3 with a stone floor at y=64 under a night sky.
fn dark_floor_world(difficulty: Difficulty) -> (ServerWorld, LevelId) {
    static CONFIG: OnceLock<crate::world::ServerWorldConfig> = OnceLock::new();
    let mut world =
        ServerWorld::new(CONFIG.get_or_init(crate::tests::datapack_config).clone()).unwrap();
    let dimension = world.require_level_id(&OVERWORLD.into());
    let stone = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID,
    )
    .unwrap();
    for cx in -3..=3 {
        for cz in -3..=3 {
            world
                .load_entity_chunk_for_test(OVERWORLD, (cx, cz))
                .unwrap();
            let level = world.levels.get_mut(&dimension).unwrap();
            level.chunks.load_column(cx, cz);
            for x in cx * 16..cx * 16 + 16 {
                for z in cz * 16..cz * 16 + 16 {
                    level.chunks.set_block_state(x, 64, z, stone);
                }
            }
        }
    }
    world.admin.set_difficulty(difficulty);
    let level = world.levels.get_mut(&dimension).unwrap();
    level.chunks.flush_live_relights().unwrap();
    level.sky_darkening = Some(15);
    level.entities.set_difficulty(difficulty);
    (world, dimension)
}

fn spawn(world: &mut ServerWorld, dimension: &LevelId, entity_type_id: i32, x: f64) -> i32 {
    world
        .levels
        .get_mut(dimension)
        .unwrap()
        .entities
        .spawn_general_living(entity_type_id, Vec3d { x, y: 65.0, z: 0.5 })
        .unwrap()
}

fn ids_of(world: &ServerWorld, dimension: &LevelId, entity_type_id: i32) -> Vec<i32> {
    let entities = &world.levels.get(dimension).unwrap().entities;
    entities
        .ids()
        .into_iter()
        .filter(|id| {
            entities.get(*id).is_some_and(|entity| {
                entity.entity_type_id() == entity_type_id && entity.removal_reason().is_none()
            })
        })
        .collect()
}

fn record_field(
    world: &ServerWorld,
    dimension: &LevelId,
    entity_id: i32,
    name: &str,
) -> Option<NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = world
        .levels
        .get(dimension)
        .unwrap()
        .entities
        .command_data(entity_id)
        .unwrap()
    else {
        panic!("an entity record is a compound")
    };
    entries
        .into_iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value)
}

/// `/data merge entity`: the record with `fields` replaced.
fn merge_record(entities: &mut EntityStore, entity_id: i32, fields: Vec<NbtSummaryEntry>) {
    let NbtSummaryValue::Compound(mut entries) = entities.command_data(entity_id).unwrap() else {
        panic!("an entity record is a compound")
    };
    for field in fields {
        entries.retain(|entry| entry.name != field.name);
        entries.push(field);
    }
    entities.load_command_data(entity_id, &entries).unwrap();
}

#[test]
fn a_hurt_zombie_on_hard_calls_a_reinforcement_that_targets_its_attacker() {
    let (mut world, dimension) = dark_floor_world(Difficulty::Hard);
    let zombie = spawn(&mut world, &dimension, ZOMBIE, 0.5);
    let pig = spawn(&mut world, &dimension, VANILLA_ENTITY_TYPE_PIG_ID, 2.5);
    let pig_uuid = world
        .levels
        .get(&dimension)
        .unwrap()
        .entities
        .get(pig)
        .unwrap()
        .uuid();
    world
        .levels
        .get_mut(&dimension)
        .unwrap()
        .entities
        .get_mut(zombie)
        .unwrap()
        .living_mut()
        .unwrap()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)
        .unwrap()
        .set_base_value(1.0);
    // Each blow searches 50 random cells; a few blows make a miss vanishingly rare.
    for _ in 0..8 {
        world
            .hurt_stored_entity_by_command_for_test(
                OVERWORLD,
                zombie,
                "minecraft:mob_attack",
                0.5,
                Some(crate::commands::CommandDamageSourceEntity::Stored {
                    entity_id: pig,
                    uuid: Some(pig_uuid),
                    position: Some(Vec3d::default()),
                    entity_resource_id: "minecraft:pig".to_string(),
                    is_mob: true,
                    is_living_non_player: true,
                    display_name: NbtSummaryValue::string("minecraft:pig"),
                }),
            )
            .expect("the zombie takes command damage");
        if ids_of(&world, &dimension, ZOMBIE).len() > 1 {
            break;
        }
    }
    let reinforcements: Vec<_> = ids_of(&world, &dimension, ZOMBIE)
        .into_iter()
        .filter(|id| *id != zombie)
        .collect();
    assert_eq!(reinforcements.len(), 1);
    let entities = &world.levels.get(&dimension).unwrap().entities;
    let reinforcement = entities.get(reinforcements[0]).unwrap();
    let offset = reinforcement.position();
    assert!(offset.x.abs().max(offset.z.abs()) >= 7.0 && offset.y == 65.0);
    assert_eq!(entities.mob_target(reinforcements[0]), Some(pig_uuid));
    let charge = |id: i32, modifier: &str| {
        entities
            .get(id)
            .unwrap()
            .living()
            .unwrap()
            .attributes()
            .instance(VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID)
            .unwrap()
            .modifier(modifier)
            .map(|modifier| modifier.amount)
    };
    assert_eq!(charge(zombie, REINFORCEMENT_CALLER_CHARGE), Some(-0.05));
    assert_eq!(
        charge(reinforcements[0], REINFORCEMENT_CALLEE_CHARGE),
        Some(-0.05)
    );
}

#[test]
fn a_zombie_bite_on_hard_turns_a_villager_into_a_zombie_villager_with_its_trades() {
    let (mut world, dimension) = dark_floor_world(Difficulty::Hard);
    let zombie = spawn(&mut world, &dimension, ZOMBIE, 0.5);
    let villager = spawn(&mut world, &dimension, VANILLA_ENTITY_TYPE_VILLAGER_ID, 1.5);
    {
        let entities = &mut world.levels.get_mut(&dimension).unwrap().entities;
        let offer = MerchantOffer::new(
            MerchantItemCost::new("minecraft:wheat".into(), 20, Vec::new()).unwrap(),
            None,
            plain_stack("minecraft:emerald", 1).unwrap(),
            16,
            2,
            0.05,
        );
        let body = entities.get_mut(villager).unwrap();
        assert!(body.initialize_merchant_offers(MerchantOffers::new(vec![offer])));
        merge_record(
            entities,
            villager,
            vec![
                NbtSummaryEntry {
                    name: "VillagerData".into(),
                    value: NbtSummaryValue::Compound(vec![
                        NbtSummaryEntry {
                            name: "type".into(),
                            value: NbtSummaryValue::String("minecraft:taiga".into()),
                        },
                        NbtSummaryEntry {
                            name: "profession".into(),
                            value: NbtSummaryValue::String("minecraft:farmer".into()),
                        },
                        NbtSummaryEntry {
                            name: "level".into(),
                            value: NbtSummaryValue::Int(2),
                        },
                    ]),
                },
                NbtSummaryEntry {
                    name: "Xp".into(),
                    value: NbtSummaryValue::Int(14),
                },
            ],
        );
    }
    let villager_data = record_field(&world, &dimension, villager, "VillagerData");
    let offers = record_field(&world, &dimension, villager, "Offers");
    assert!(offers.is_some());

    // The killing blow lands while the attacker is lifted out of the store,
    // the way a melee goal swings.
    let players = world.nearby_players_by_key(&dimension);
    let mut leftover =
        crate::world::simulation_level::pending_spawns::PendingEntitySpawns::default();
    world.with_level_ctx(&dimension, &players, &mut leftover, |level, rest| {
        let body = rest.entities.detach(zombie).unwrap();
        let hit = crate::entity::ai::mob::brain_runtime::hurt_entity_from_body(
            &body,
            rest.entities,
            level,
            villager,
            "minecraft:mob_attack",
            1000.0,
        );
        rest.entities.attach(body);
        assert_eq!(hit, Some(true));
    });
    world.run_entity_commands(&dimension, &mut Vec::new());

    assert!(ids_of(&world, &dimension, VANILLA_ENTITY_TYPE_VILLAGER_ID).is_empty());
    let infected = ids_of(&world, &dimension, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID);
    assert_eq!(infected.len(), 1);
    assert_eq!(
        record_field(&world, &dimension, infected[0], "VillagerData"),
        villager_data
    );
    assert_eq!(
        record_field(&world, &dimension, infected[0], "Offers"),
        offers
    );
    assert_eq!(
        record_field(&world, &dimension, infected[0], "Xp"),
        Some(NbtSummaryValue::Int(14))
    );
}
