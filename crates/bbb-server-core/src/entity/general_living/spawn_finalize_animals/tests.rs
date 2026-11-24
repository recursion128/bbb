use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::NbtSummaryValue;

use super::super::spawn_finalize::test_level::TestLevel;
use super::super::spawn_finalize::{AgeableGroup, AgeableKind, SpawnGroupData, SpawnReason};
use crate::entity::{EntityData, EntityIdCounter, EntityStore, GeneralLivingData};

fn data(level: &TestLevel, id: i32) -> &GeneralLivingData {
    let EntityData::GeneralLiving(data) = level.get(id).data() else {
        panic!("GeneralLiving")
    };
    data
}

fn is_baby(level: &TestLevel, id: i32) -> bool {
    data(level, id)
        .ageable_retained()
        .is_some_and(|age| age.is_baby())
}

fn saved_field(store: &EntityStore, name: &str) -> Option<NbtSummaryValue> {
    let saved = crate::entity::save::write_entity_chunk(store, (0, -1)).unwrap();
    let record = saved.entities().next().unwrap();
    record
        .raw()
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.value.clone())
}

#[test]
fn farm_animal_saves_the_variant_its_spawn_selected() {
    let mut level = TestLevel::new()
        .variant("minecraft:chicken_variant", "minecraft:warm", 7)
        .variant("minecraft:chicken_sound_variant", "minecraft:classic", 0);
    let (id, _) = level.spawn("minecraft:chicken", SpawnReason::Spawner, None, 1);
    assert_eq!(
        data(&level, id).registry_variant("minecraft:chicken_variant"),
        Some("minecraft:warm")
    );
    assert_eq!(
        saved_field(&level.store, "variant"),
        Some(NbtSummaryValue::String("minecraft:warm".into()))
    );
}

#[test]
fn cat_variant_sound_and_persistence_round_trip() {
    for with_variant in [false, true] {
        let mut level =
            TestLevel::new().variant("minecraft:cat_sound_variant", "minecraft:royal", 1);
        if with_variant {
            level = level.variant("minecraft:cat_variant", "minecraft:tabby", 7);
        }
        let mut cat = level.create("minecraft:cat");
        crate::entity::finalize_spawn(
            &mut level,
            &mut cat,
            SpawnReason::Natural,
            None,
            &crate::random::GameRandom::with_seed(3),
        )
        .unwrap();
        assert!(crate::entity::set_cat_spawn_persistence(&mut cat));
        level.store.add_fresh(cat);
        let variant = if with_variant {
            "minecraft:tabby"
        } else {
            "minecraft:black"
        };
        assert_eq!(
            saved_field(&level.store, "variant"),
            Some(NbtSummaryValue::String(variant.into()))
        );
        let saved = crate::entity::save::write_entity_chunk(&level.store, (0, -1)).unwrap();
        let mut restored = EntityStore::new(2, EntityIdCounter::new());
        crate::entity::save::read_entity_chunk(&mut restored, &saved).unwrap();
        for name in ["variant", "sound_variant", "PersistenceRequired"] {
            assert_eq!(
                saved_field(&restored, name),
                saved_field(&level.store, name),
                "{name}"
            );
        }
        assert_eq!(
            saved_field(&restored, "sound_variant"),
            Some(NbtSummaryValue::String("minecraft:royal".into()))
        );
    }
}

#[test]
fn axolotl_group_shares_its_variants_and_the_third_member_is_a_baby() {
    let mut level = TestLevel::new();
    let (first, group) = level.spawn("minecraft:axolotl", SpawnReason::Natural, None, 1);
    let (second, group) = level.spawn("minecraft:axolotl", SpawnReason::Natural, group, 2);
    let variants = match &group {
        Some(SpawnGroupData::Ageable(AgeableGroup {
            kind: AgeableKind::Axolotl(variants),
            ..
        })) => *variants,
        other => panic!("axolotl group {other:?}"),
    };
    let (third, _) = level.spawn("minecraft:axolotl", SpawnReason::Natural, group, 3);
    assert!(!is_baby(&level, first));
    assert!(!is_baby(&level, second));
    assert!(is_baby(&level, third));
    assert!(variants.iter().all(|variant| (0..4).contains(variant)));
}

#[test]
fn fox_variant_follows_the_snow_biome_tag() {
    for snow in [false, true] {
        let mut level = TestLevel::new();
        if snow {
            level.ctx.biome_tags.push("minecraft:spawns_snow_foxes");
        }
        let (id, _) = level.spawn("minecraft:fox", SpawnReason::Spawner, None, 5);
        assert_eq!(
            crate::entity::general_living::fox_target_goals(level.get(id)),
            Some(if snow {
                crate::entity::fox::FoxVariant::Snow
            } else {
                crate::entity::fox::FoxVariant::Red
            })
        );
    }
}

#[test]
fn natural_zombie_horse_brings_a_spear_zombie_rider() {
    let mut level = TestLevel::new();
    let (horse, _) = level.spawn("minecraft:zombie_horse", SpawnReason::Natural, None, 1);
    let riders = level.store.passengers(horse).to_vec();
    assert_eq!(riders.len(), 1);
    let rider = level.get(riders[0]);
    assert_eq!(rider.data().entity_resource_id(), "minecraft:zombie");
    assert_eq!(
        rider
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::MainHand)
            .and_then(|stack| stack.item_resource_id()),
        Some("minecraft:iron_spear")
    );

    let (horse, _) = level.spawn(
        "minecraft:zombie_horse",
        SpawnReason::ChunkGeneration,
        None,
        1,
    );
    assert!(level.store.passengers(horse).is_empty());
}

#[test]
fn school_members_follow_the_first_fish_and_share_its_variant() {
    let mut level = TestLevel::new();
    let (leader, group) = level.spawn("minecraft:cod", SpawnReason::Natural, None, 1);
    let (follower, _) = level.spawn("minecraft:cod", SpawnReason::Natural, group, 2);
    let leader_uuid = level.get(leader).uuid();
    assert_eq!(
        crate::entity::schooling_fish_state(level.get(follower))
            .and_then(|state| state.leader_uuid()),
        Some(leader_uuid)
    );

    let (leader, group) = (0..64)
        .map(|seed| level.spawn("minecraft:tropical_fish", SpawnReason::Natural, None, seed))
        .find(|(_, group)| {
            matches!(
                group,
                Some(SpawnGroupData::School {
                    tropical: Some(_),
                    ..
                })
            )
        })
        .expect("a common tropical fish leads a school");
    let (follower, _) = level.spawn("minecraft:tropical_fish", SpawnReason::Natural, group, 9);
    let variant = |id| {
        let EntityData::GeneralLiving(data) = level.get(id).data() else {
            panic!("fish")
        };
        data.fish_leaf()
    };
    assert_eq!(variant(follower), variant(leader));
}
