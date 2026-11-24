use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_FOX_ID, VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID,
    VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_FOX_SPAWN_EGG_ID, VANILLA_ITEM_GOAT_SPAWN_EGG_ID,
    VANILLA_ITEM_HOGLIN_SPAWN_EGG_ID, VANILLA_ITEM_PARROT_SPAWN_EGG_ID,
    VANILLA_ITEM_VILLAGER_SPAWN_EGG_ID, VANILLA_ITEM_WOLF_SPAWN_EGG_ID,
    VANILLA_ITEM_ZOMBIE_SPAWN_EGG_ID,
};
use bbb_protocol::packets::Vec3d;

use super::*;
use crate::entity::general_living::GeneralLivingData;
use crate::entity::store::EntityIdCounter;

const PLAYER: Uuid = Uuid::from_u128(0xe66);

fn spawn(store: &mut EntityStore, entity_type_id: i32) -> i32 {
    store
        .spawn_general_living(
            entity_type_id,
            Vec3d {
                x: 0.5,
                y: 64.0,
                z: 0.5,
            },
        )
        .expect("a stored mob")
}

fn use_egg(
    store: &mut EntityStore,
    parent: i32,
    egg_item_id: i32,
    level: Option<&SpawnEggLevel>,
    seed: i64,
) -> (SpawnEggOffspring, Option<i32>, ItemStackSummary) {
    let mut hand = ItemStackSummary {
        item_id: Some(egg_item_id),
        count: 1,
        component_patch: Default::default(),
    };
    let outcome = spawn_offspring_from_spawn_egg(
        store,
        parent,
        SpawnEggUser {
            hand_stack: &mut hand,
            infinite_materials: false,
            uuid: PLAYER,
        },
        level,
        &EntityRegistryTagView::Builtin26_1,
        &GameRandom::with_seed(seed),
    );
    let baby = store.ids().into_iter().find(|id| *id != parent);
    (outcome, baby, hand)
}

fn data(store: &EntityStore, id: i32) -> &GeneralLivingData {
    match store.get(id).unwrap().data() {
        EntityData::GeneralLiving(data) => data,
        _ => unreachable!(),
    }
}

/// `Wolf.getBreedOffspring` (`Wolf.java:580-600`) with the parent as partner.
#[test]
fn a_tamed_wolf_egg_baby_shares_the_owner_and_collar() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    store.set_item_publication(crate::entity::ItemPublication::builtin().clone());
    let wolf = spawn(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID);
    let parent = store.get_mut(wolf).unwrap();
    assert!(crate::entity::taming::set_tame_offspring(
        parent,
        Some(PLAYER)
    ));
    assert!(crate::entity::collar::set_collar_color(parent, 11));

    let (outcome, baby, hand) = use_egg(&mut store, wolf, VANILLA_ITEM_WOLF_SPAWN_EGG_ID, None, 3);
    assert_eq!(outcome, SpawnEggOffspring::Spawned);
    assert_eq!(hand.count, 0);
    let baby = baby.unwrap();
    let tamable = data(&store, baby).tamable_retained().unwrap();
    assert!(tamable.is_tame());
    assert_eq!(tamable.owner(), Some(PLAYER));
    assert_eq!(
        crate::entity::collar::color_of(store.get(baby).unwrap()),
        Some(11)
    );
    assert!(data(&store, baby).is_baby());
}

#[test]
fn a_typed_wolf_egg_overrides_variant_and_collar_and_preserves_taming() {
    use bbb_protocol::builtin_enum_component_values::VANILLA_DYE_COLOR_WHITE_ID;
    use bbb_protocol::data_component_types::{
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID, VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
    };

    let mut store = EntityStore::new(1, EntityIdCounter::new());
    store.set_item_publication(crate::entity::ItemPublication::builtin().clone());
    let wolf = spawn(&mut store, VANILLA_ENTITY_TYPE_WOLF_ID);
    let parent = store.get_mut(wolf).unwrap();
    assert!(crate::entity::taming::set_tame_offspring(
        parent,
        Some(PLAYER)
    ));
    assert!(crate::entity::collar::set_collar_color(parent, 11));
    let mut hand = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_WOLF_SPAWN_EGG_ID),
        count: 1,
        component_patch: Default::default(),
    };
    hand.component_patch.added_type_ids = vec![
        VANILLA_DATA_COMPONENT_WOLF_VARIANT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
    ];
    hand.component_patch.wolf_variant_id = Some(
        store
            .item_publication()
            .resolver()
            .holder_id("minecraft:wolf_variant", "minecraft:black")
            .unwrap(),
    );
    hand.component_patch.builtin_enum_component_ids.insert(
        VANILLA_DATA_COMPONENT_WOLF_COLLAR_TYPE_ID,
        VANILLA_DYE_COLOR_WHITE_ID,
    );

    assert_eq!(
        spawn_offspring_from_spawn_egg(
            &mut store,
            wolf,
            SpawnEggUser {
                hand_stack: &mut hand,
                infinite_materials: false,
                uuid: PLAYER
            },
            None,
            &EntityRegistryTagView::Builtin26_1,
            &GameRandom::with_seed(3),
        ),
        SpawnEggOffspring::Spawned,
    );
    let baby = store.ids().into_iter().find(|id| *id != wolf).unwrap();
    let tamable = data(&store, baby).tamable_retained().unwrap();
    assert!(tamable.is_tame());
    assert_eq!(tamable.owner(), Some(PLAYER));
    assert_eq!(
        data(&store, baby).registry_variant("minecraft:wolf_variant"),
        Some("minecraft:black"),
    );
    assert_eq!(
        crate::entity::collar::color_of(store.get(baby).unwrap()),
        Some(VANILLA_DYE_COLOR_WHITE_ID as i8),
    );
    assert!(data(&store, baby).is_baby());
    assert_eq!(hand.count, 0);
}

/// `Goat.getBreedOffspring` (`Goat.java:145-152`).
#[test]
fn a_screaming_goat_egg_baby_screams() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let goat = spawn(&mut store, VANILLA_ENTITY_TYPE_GOAT_ID);
    assert!(crate::entity::goat::apply_spawn_traits(
        store.get_mut(goat).unwrap(),
        true,
        crate::entity::goat::SpawnHornRemoval::None,
    ));
    let (outcome, baby, _) = use_egg(&mut store, goat, VANILLA_ITEM_GOAT_SPAWN_EGG_ID, None, 5);
    assert_eq!(outcome, SpawnEggOffspring::Spawned);
    assert!(data(&store, baby.unwrap())
        .goat_retained()
        .unwrap()
        .is_screaming());
}

/// `Hoglin.getBreedOffspring` sets persistence (`Hoglin.java:298-305`).
#[test]
fn a_hoglin_egg_baby_is_persistent() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let hoglin = spawn(&mut store, VANILLA_ENTITY_TYPE_HOGLIN_ID);
    let (_, baby, _) = use_egg(
        &mut store,
        hoglin,
        VANILLA_ITEM_HOGLIN_SPAWN_EGG_ID,
        None,
        1,
    );
    let baby = baby.unwrap();
    assert!(data(&store, baby)
        .mob
        .as_ref()
        .unwrap()
        .persistence_required());
    assert!(data(&store, baby).is_baby());
}

/// `Villager.getBreedOffspring` (`Villager.java:741-753`): the biome's type or
/// the parent's.
#[test]
fn a_villager_egg_baby_takes_the_biome_or_parent_type() {
    let level = SpawnEggLevel {
        biome: Some("minecraft:snowy_plains".into()),
        special_multiplier: 0.0,
    };
    let mut seen = std::collections::BTreeSet::new();
    for seed in 0..16 {
        let mut store = EntityStore::new(1, EntityIdCounter::new());
        let villager = spawn(&mut store, VANILLA_ENTITY_TYPE_VILLAGER_ID);
        assert!(
            crate::entity::general_living::finalize_spawner_villager_type(
                store.get_mut(villager).unwrap(),
                "minecraft:desert",
            )
        );
        let (_, baby, _) = use_egg(
            &mut store,
            villager,
            VANILLA_ITEM_VILLAGER_SPAWN_EGG_ID,
            Some(&level),
            seed,
        );
        let baby = baby.unwrap();
        assert!(data(&store, baby).is_baby());
        seen.insert(
            data(&store, baby)
                .component_villager_type()
                .unwrap()
                .to_owned(),
        );
    }
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        ["minecraft:desert", "minecraft:snow"]
    );
}

/// `Parrot.getBreedOffspring` is null (`Parrot.java:322-324`): nothing spawns
/// and the egg is kept.
#[test]
fn a_parrot_egg_on_a_parrot_spawns_nothing() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let parrot = spawn(&mut store, VANILLA_ENTITY_TYPE_PARROT_ID);
    let (outcome, baby, hand) = use_egg(
        &mut store,
        parrot,
        VANILLA_ITEM_PARROT_SPAWN_EGG_ID,
        None,
        1,
    );
    assert_eq!(outcome, SpawnEggOffspring::Pass);
    assert_eq!(baby, None);
    assert_eq!(hand.count, 1);
}

/// `Fox.onOffspringSpawnedFromEgg` (`Fox.java:591-593`).
#[test]
fn a_fox_egg_baby_trusts_the_player() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let fox = spawn(&mut store, VANILLA_ENTITY_TYPE_FOX_ID);
    let (_, baby, _) = use_egg(&mut store, fox, VANILLA_ITEM_FOX_SPAWN_EGG_ID, None, 1);
    assert!(data(&store, baby.unwrap())
        .fox_retained()
        .unwrap()
        .trusts_uuid(PLAYER));
}

/// `Zombie.onOffspringSpawnedFromEgg` (`Zombie.java:508-513`): loot pickup at
/// `0.55 * getSpecialMultiplier`, never at zero.
#[test]
fn a_zombie_egg_baby_rolls_loot_pickup_from_the_special_multiplier() {
    let pickups = |special_multiplier: f32| {
        let level = SpawnEggLevel {
            biome: None,
            special_multiplier,
        };
        (0..20)
            .map(|seed| {
                let mut store = EntityStore::new(1, EntityIdCounter::new());
                let zombie = spawn(&mut store, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
                let (_, baby, _) = use_egg(
                    &mut store,
                    zombie,
                    VANILLA_ITEM_ZOMBIE_SPAWN_EGG_ID,
                    Some(&level),
                    seed,
                );
                data(&store, baby.unwrap())
                    .mob
                    .as_ref()
                    .unwrap()
                    .can_pick_up_loot()
            })
            .filter(|&pick_up| pick_up)
            .count()
    };
    assert_eq!(pickups(0.0), 0);
    let some = pickups(1.0);
    assert!(some > 0 && some < 20, "{some}");
}
