use super::*;
use bbb_protocol::entity_types::*;

#[test]
fn unset_values_use_path_type_defaults() {
    let malus = PathfindingMalus::default();
    assert!(malus.values.iter().all(Option::is_none));
    for path_type in [
        GroundPathType::Blocked,
        GroundPathType::Open,
        GroundPathType::Water,
        GroundPathType::Fire,
        GroundPathType::Leaves,
        GroundPathType::BigMobsCloseToDanger,
    ] {
        assert_eq!(malus.get(path_type), path_type.default_malus());
    }
    for entity_type in [
        VANILLA_ENTITY_TYPE_PILLAGER_ID,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        VANILLA_ENTITY_TYPE_EVOKER_ID,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
        VANILLA_ENTITY_TYPE_WITCH_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ] {
        assert_eq!(PathfindingMalus::for_entity_type(entity_type), malus);
    }
}

#[test]
fn constructor_overrides_follow_the_class_chain() {
    use GroundPathType::*;
    const ANIMAL: &[(GroundPathType, f32)] = &[(FireInNeighbor, 16.0), (Fire, -1.0)];
    let animals = [
        VANILLA_ENTITY_TYPE_COW_ID,
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_SHEEP_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAT_ID,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
    ]
    .map(|id| (id, ANIMAL.to_vec()));
    let rows = [
        (VANILLA_ENTITY_TYPE_RAVAGER_ID, vec![(Leaves, 0.0)]),
        (
            VANILLA_ENTITY_TYPE_FROG_ID,
            vec![
                (FireInNeighbor, 16.0),
                (Fire, -1.0),
                (Water, 4.0),
                (Trapdoor, -1.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_PARROT_ID,
            vec![(FireInNeighbor, -1.0), (Fire, -1.0), (Cocoa, -1.0)],
        ),
        (
            VANILLA_ENTITY_TYPE_BEE_ID,
            vec![
                (FireInNeighbor, -1.0),
                (Fire, -1.0),
                (Water, -1.0),
                (WaterBorder, 16.0),
                (Cocoa, -1.0),
                (Fence, -1.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_STRIDER_ID,
            vec![
                (Water, -1.0),
                (Lava, 0.0),
                (FireInNeighbor, 0.0),
                (Fire, 0.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
            vec![
                (FireInNeighbor, 16.0),
                (DamagingInNeighbor, 16.0),
                (Fire, -1.0),
            ],
        ),
        (VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, vec![]),
        (
            VANILLA_ENTITY_TYPE_WOLF_ID,
            vec![
                (FireInNeighbor, 16.0),
                (Fire, -1.0),
                (PowderSnow, -1.0),
                (OnTopOfPowderSnow, -1.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_GOAT_ID,
            vec![
                (FireInNeighbor, 16.0),
                (Fire, -1.0),
                (PowderSnow, -1.0),
                (OnTopOfPowderSnow, -1.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_SNIFFER_ID,
            vec![
                (FireInNeighbor, 16.0),
                (Fire, -1.0),
                (Water, -1.0),
                (OnTopOfPowderSnow, -1.0),
                (DamageCautious, -1.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_FOX_ID,
            vec![
                (FireInNeighbor, 16.0),
                (Fire, -1.0),
                (DamagingInNeighbor, 0.0),
                (Damaging, 0.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            vec![(FireInNeighbor, 16.0), (Fire, -1.0), (Water, 0.0)],
        ),
        (
            VANILLA_ENTITY_TYPE_TURTLE_ID,
            vec![
                (FireInNeighbor, 16.0),
                (Fire, -1.0),
                (Water, 0.0),
                (DoorIronClosed, -1.0),
                (DoorWoodClosed, -1.0),
                (DoorOpen, -1.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            vec![(FireInNeighbor, 16.0), (Fire, -1.0), (Water, 0.0)],
        ),
        (VANILLA_ENTITY_TYPE_DOLPHIN_ID, vec![(Water, 0.0)]),
        (VANILLA_ENTITY_TYPE_COD_ID, vec![(Water, 0.0)]),
        (VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, vec![(Water, 0.0)]),
        (VANILLA_ENTITY_TYPE_DROWNED_ID, vec![(Water, 0.0)]),
        (VANILLA_ENTITY_TYPE_ENDERMAN_ID, vec![(Water, -1.0)]),
        (VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, vec![(Lava, 8.0)]),
        (VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID, vec![(Lava, 8.0)]),
        (
            VANILLA_ENTITY_TYPE_BLAZE_ID,
            vec![
                (Water, -1.0),
                (Lava, 8.0),
                (FireInNeighbor, 0.0),
                (Fire, 0.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_WARDEN_ID,
            vec![
                (UnpassableRail, 0.0),
                (Damaging, 8.0),
                (PowderSnow, 8.0),
                (Lava, 8.0),
                (Fire, 0.0),
                (FireInNeighbor, 0.0),
            ],
        ),
        (
            VANILLA_ENTITY_TYPE_BREEZE_ID,
            vec![(OnTopOfTrapdoor, -1.0), (Fire, -1.0)],
        ),
    ];
    for (entity_type, overrides) in animals.into_iter().chain(rows) {
        let malus = PathfindingMalus::for_entity_type(entity_type);
        assert_eq!(
            malus.values.iter().filter(|entry| entry.is_some()).count(),
            overrides.len(),
            "{entity_type}"
        );
        for (path_type, expected) in overrides {
            assert_eq!(
                malus.get(path_type),
                expected,
                "{entity_type} {path_type:?}"
            );
        }
    }
}
