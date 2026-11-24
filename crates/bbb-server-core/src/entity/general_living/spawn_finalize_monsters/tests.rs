use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::equipment_slots::EquipmentSlot;

use super::super::spawn_finalize::test_level::TestLevel;
use super::super::spawn_finalize::{SpawnGroupData, SpawnReason};
use crate::entity::ai::{MemoryGlobalPos, MemoryModuleType, MemoryValue};
use crate::entity::natural_spawn_facts_26_1::VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1;
use crate::entity::{EntityData, EntityLifecycleEvent, GeneralLivingData};

fn data(level: &TestLevel, id: i32) -> &GeneralLivingData {
    let EntityData::GeneralLiving(data) = level.get(id).data() else {
        panic!("GeneralLiving")
    };
    data
}

fn main_hand(level: &TestLevel, id: i32) -> Option<&'static str> {
    data(level, id)
        .living()
        .item_by_slot(EquipmentSlot::MainHand)
        .and_then(|stack| stack.item_resource_id())
}

#[test]
fn every_natural_spawn_type_finalizes_for_each_reason() {
    let owners = VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1
        .iter()
        .filter(|facts| facts.is_mob);
    for facts in owners {
        for reason in [
            SpawnReason::Natural,
            SpawnReason::ChunkGeneration,
            SpawnReason::Spawner,
            SpawnReason::Structure,
        ] {
            let mut level = TestLevel::new();
            for (registry, resource_id) in [
                ("minecraft:cat_variant", "minecraft:tabby"),
                ("minecraft:cat_sound_variant", "minecraft:classic"),
                ("minecraft:chicken_variant", "minecraft:temperate"),
                ("minecraft:chicken_sound_variant", "minecraft:classic"),
                ("minecraft:cow_variant", "minecraft:temperate"),
                ("minecraft:cow_sound_variant", "minecraft:classic"),
                ("minecraft:frog_variant", "minecraft:temperate"),
                ("minecraft:pig_variant", "minecraft:temperate"),
                ("minecraft:pig_sound_variant", "minecraft:classic"),
                ("minecraft:wolf_variant", "minecraft:woods"),
                ("minecraft:zombie_nautilus_variant", "minecraft:temperate"),
            ] {
                level = level.variant(registry, resource_id, 1);
            }
            let mut entity = level.create(facts.resource_id);
            assert!(
                crate::entity::finalize_spawn(
                    &mut level,
                    &mut entity,
                    reason,
                    None,
                    &crate::random::GameRandom::with_seed(1),
                )
                .is_some(),
                "{} {reason:?}",
                facts.resource_id
            );
        }
    }
}

#[test]
fn spider_group_members_share_the_first_members_effect() {
    let mut level = TestLevel::new();
    let group = Some(SpawnGroupData::Spider {
        effect: Some("minecraft:speed"),
    });
    let (id, out) = level.spawn("minecraft:spider", SpawnReason::Natural, group.clone(), 1);
    assert_eq!(out, group);
    assert!(data(&level, id)
        .living()
        .active_effect("minecraft:speed")
        .is_some());
    // Below Hard nothing starts a group effect.
    let (id, out) = level.spawn("minecraft:spider", SpawnReason::Natural, None, 1);
    assert_eq!(out, Some(SpawnGroupData::Spider { effect: None }));
    assert_eq!(data(&level, id).living().active_effects().count(), 0);
}

#[test]
fn zombie_group_baby_flag_carries_to_every_member() {
    let mut level = TestLevel::new();
    let group = Some(SpawnGroupData::Zombie {
        baby: true,
        can_spawn_jockey: false,
    });
    for seed in 0..4 {
        let (id, out) = level.spawn(
            "minecraft:zombie",
            SpawnReason::Natural,
            group.clone(),
            seed,
        );
        assert_eq!(out, group);
        assert!(data(&level, id)
            .zombie_retained()
            .is_some_and(|zombie| zombie.is_baby()));
    }
}

#[test]
fn only_natural_and_structure_trident_drowned_ride_a_nautilus() {
    for reason in [SpawnReason::Natural, SpawnReason::Spawner] {
        let mut level = TestLevel::new().variant(
            "minecraft:zombie_nautilus_variant",
            "minecraft:temperate",
            1,
        );
        let mut ridden = 0;
        for seed in 0..2_000 {
            let (id, _) = level.spawn("minecraft:drowned", reason, None, seed);
            let Some(vehicle) = level.store.vehicle(id) else {
                continue;
            };
            ridden += 1;
            assert_eq!(
                level.get(vehicle).data().entity_resource_id(),
                "minecraft:zombie_nautilus"
            );
            assert_eq!(main_hand(&level, id), Some("minecraft:trident"));
        }
        assert_eq!(ridden > 0, reason == SpawnReason::Natural, "{reason:?}");
    }
}

#[test]
fn natural_raiders_can_join_raids_except_natural_witches() {
    for resource_id in ["minecraft:pillager", "minecraft:witch"] {
        for reason in [SpawnReason::Natural, SpawnReason::ChunkGeneration] {
            let mut level = TestLevel::new();
            let (id, _) = level.spawn(resource_id, reason, None, 1);
            let state = *data(&level, id).raider_retained().unwrap();
            assert_eq!(
                state.can_join_raid(),
                resource_id != "minecraft:witch" || reason != SpawnReason::Natural
            );
        }
    }
}

#[test]
fn patrol_leader_wears_the_ominous_banner() {
    let mut level = TestLevel::new();
    level
        .store
        .set_item_publication(crate::entity::ItemPublication::builtin().clone());
    let mut pillager = level.create("minecraft:pillager");
    assert!(crate::entity::set_patrol_leader(&mut pillager));
    crate::entity::finalize_spawn(
        &mut level,
        &mut pillager,
        SpawnReason::Patrol,
        None,
        &crate::random::GameRandom::with_seed(1),
    )
    .unwrap();
    let id = level.store.add_fresh(pillager);
    let data = data(&level, id);
    assert_eq!(
        data.living()
            .item_by_slot(EquipmentSlot::Head)
            .and_then(|stack| stack.item_resource_id()),
        Some("minecraft:white_banner")
    );
    assert_eq!(
        data.mob.as_ref().unwrap().drop_chance(EquipmentSlot::Head),
        2.0
    );
    assert_eq!(main_hand(&level, id), Some("minecraft:crossbow"));
    let (_, leader, patrolling) = crate::entity::patrol_spawn_state(level.get(id)).unwrap();
    assert!(leader && patrolling);
}

#[test]
fn structure_piglin_keeps_its_loaded_baby_and_weapon() {
    let mut level = TestLevel::new();
    let mut piglin = level.create("minecraft:piglin");
    assert!(crate::entity::set_piglin_baby(&mut piglin, true));
    assert!(crate::entity::set_spawner_item(
        &mut piglin,
        EquipmentSlot::MainHand,
        "minecraft:golden_spear"
    ));
    crate::entity::finalize_spawn(
        &mut level,
        &mut piglin,
        SpawnReason::Structure,
        None,
        &crate::random::GameRandom::with_seed(1),
    )
    .unwrap();
    let id = level.store.add_fresh(piglin);
    assert_eq!(data(&level, id).piglin_or_zoglin_baby(), Some(true));
    assert_eq!(main_hand(&level, id), Some("minecraft:golden_spear"));
}

#[test]
fn piglin_brute_home_is_its_own_dimension() {
    let mut level = TestLevel::new();
    level.ctx.dimension = "bbb:custom_level".into();
    let (id, _) = level.spawn("minecraft:piglin_brute", SpawnReason::Spawner, None, 1);
    assert_eq!(
        crate::entity::spawner_brain_memory(level.get(id), MemoryModuleType::Home)
            .map(|memory| &memory.value),
        Some(&MemoryValue::GlobalPos(MemoryGlobalPos {
            dimension: "bbb:custom_level".to_owned(),
            pos: level.get(id).block_position(),
        }))
    );
    assert_eq!(main_hand(&level, id), Some("minecraft:golden_axe"));
}

#[test]
fn villager_type_follows_the_biome() {
    let mut level = TestLevel::new();
    level.ctx.biome = Some("minecraft:mangrove_swamp".into());
    let (id, _) = level.spawn("minecraft:villager", SpawnReason::Natural, None, 1);
    assert_eq!(
        data(&level, id).villager_state().unwrap().villager_type(),
        "minecraft:swamp"
    );
}

#[test]
fn phantom_anchors_above_its_spawn_at_size_zero() {
    let mut level = TestLevel::new();
    let mut phantom = level.create("minecraft:phantom");
    assert!(crate::entity::phantom::set_size(&mut phantom, 4));
    crate::entity::finalize_spawn(
        &mut level,
        &mut phantom,
        SpawnReason::Natural,
        None,
        &crate::random::GameRandom::with_seed(1),
    )
    .unwrap();
    let id = level.store.add_fresh(phantom);
    let state = data(&level, id).phantom_retained().unwrap();
    assert_eq!(
        state.anchor(),
        Some(bbb_protocol::packets::BlockPos { x: 2, y: 75, z: -4 })
    );
    assert_eq!(state.size(), 0);
}

#[test]
fn copper_golem_plays_its_spawn_sound_unless_silent() {
    for silent in [false, true] {
        let mut level = TestLevel::new();
        let mut golem = level.create("minecraft:copper_golem");
        golem.set_silent(silent);
        crate::entity::finalize_spawn(
            &mut level,
            &mut golem,
            SpawnReason::Structure,
            None,
            &crate::random::GameRandom::with_seed(1),
        )
        .unwrap();
        let sounds = level
            .store
            .drain_lifecycle_events()
            .into_iter()
            .filter(|event| matches!(event, EntityLifecycleEvent::Sound { .. }))
            .count();
        assert_eq!(sounds, usize::from(!silent));
    }
}

#[test]
fn hard_zombies_roll_equipment_and_reinforcement_bonuses() {
    let mut level = TestLevel::new();
    level.ctx.difficulty = Difficulty::Hard;
    level.ctx.special_multiplier = 1.0;
    let mut armored = 0;
    for seed in 0..64 {
        let (id, _) = level.spawn("minecraft:zombie", SpawnReason::Spawner, None, seed);
        let living = data(&level, id).living();
        armored += usize::from(living.item_by_slot(EquipmentSlot::Head).is_some());
        let reinforcements = living
            .attributes()
            .base_value(
                bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SPAWN_REINFORCEMENTS_CHANCE_ID,
            )
            .unwrap();
        assert!((0.0..0.1).contains(&reinforcements));
    }
    assert!(armored > 0);
}

#[test]
fn zombie_villagers_spawn_with_a_random_profession_and_their_biome_type() {
    let mut level = TestLevel::new();
    level.ctx.biome = Some("minecraft:desert".into());
    let mut professions = std::collections::BTreeSet::new();
    for seed in 0..32 {
        let (id, _) = level.spawn(
            "minecraft:zombie_villager",
            SpawnReason::Natural,
            None,
            seed,
        );
        let villager = data(&level, id).villager_state().unwrap();
        assert_eq!(villager.villager_type(), "minecraft:desert");
        professions.insert(villager.profession().to_owned());
    }
    assert!(professions.len() > 3);
}

#[test]
fn infected_villagers_roll_no_gear_and_keep_their_pickup_flag() {
    let mut level = TestLevel::new();
    level.ctx.difficulty = Difficulty::Hard;
    level.ctx.special_multiplier = 1.0;
    for seed in 0..64 {
        let mut entity = level.create("minecraft:zombie_villager");
        let EntityData::GeneralLiving(body) = &mut entity.data else {
            unreachable!()
        };
        body.mob.as_mut().unwrap().set_can_pick_up_loot(true);
        crate::entity::finalize_spawn(
            &mut level,
            &mut entity,
            SpawnReason::Conversion,
            Some(SpawnGroupData::Zombie {
                baby: false,
                can_spawn_jockey: true,
            }),
            &crate::random::GameRandom::with_seed(seed),
        )
        .unwrap();
        let id = level.store.add_fresh(entity);
        let body = data(&level, id);
        assert!(body.mob.as_ref().unwrap().can_pick_up_loot());
        assert!(body.living().item_by_slot(EquipmentSlot::Head).is_none());
        assert_eq!(main_hand(&level, id), None);
    }
}
