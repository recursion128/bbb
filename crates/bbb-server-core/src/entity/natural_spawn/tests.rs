use super::*;

const ORIGIN: BlockPos = BlockPos { x: 0, y: 0, z: 0 };

fn player(id: u128) -> Uuid {
    Uuid::from_u128(id)
}

fn existing<'a>(
    category: VanillaEntityMobCategory,
    nearby_players: &'a [Uuid],
) -> ExistingNaturalSpawnEntity<'a> {
    ExistingNaturalSpawnEntity {
        category,
        position: ORIGIN,
        is_mob: true,
        persistence_required: false,
        requires_custom_persistence: false,
        cost_charge: None,
        nearby_players,
    }
}

#[test]
fn create_state_skips_persistent_mobs_but_counts_non_mobs_without_local_caps() {
    let nearby = [player(1)];
    let mut state = NaturalSpawnState::new(289);
    state.observe_existing(ExistingNaturalSpawnEntity {
        persistence_required: true,
        cost_charge: Some(2.0),
        ..existing(VanillaEntityMobCategory::Creature, &nearby)
    });
    state.observe_existing(ExistingNaturalSpawnEntity {
        requires_custom_persistence: true,
        cost_charge: Some(3.0),
        ..existing(VanillaEntityMobCategory::Ambient, &nearby)
    });
    state.observe_existing(ExistingNaturalSpawnEntity {
        category: VanillaEntityMobCategory::Monster,
        position: BlockPos { x: 1, y: 2, z: 3 },
        is_mob: false,
        persistence_required: true,
        requires_custom_persistence: true,
        cost_charge: Some(4.0),
        nearby_players: &nearby,
    });
    state.observe_existing(ExistingNaturalSpawnEntity {
        cost_charge: Some(5.0),
        ..existing(VanillaEntityMobCategory::Misc, &nearby)
    });

    assert_eq!(state.category_count(VanillaEntityMobCategory::Creature), 0);
    assert_eq!(state.category_count(VanillaEntityMobCategory::Ambient), 0);
    assert_eq!(state.category_count(VanillaEntityMobCategory::Monster), 1);
    assert_eq!(state.category_count(VanillaEntityMobCategory::Misc), 0);
    assert_eq!(
        state.local_category_count(nearby[0], VanillaEntityMobCategory::Monster),
        0
    );
    assert_eq!(state.potential().charge_count(), 1);
}

#[test]
fn global_caps_use_the_strict_vanilla_17_by_17_scaled_limit() {
    let mut state = NaturalSpawnState::new(289);
    assert_eq!(state.spawnable_chunk_count(), 289);
    for _ in 0..69 {
        state.observe_existing(existing(VanillaEntityMobCategory::Monster, &[]));
    }
    assert!(state.can_spawn_for_category_global(VanillaEntityMobCategory::Monster));
    state.observe_existing(existing(VanillaEntityMobCategory::Monster, &[]));
    assert!(!state.can_spawn_for_category_global(VanillaEntityMobCategory::Monster));
    assert!(
        !NaturalSpawnState::new(1).can_spawn_for_category_global(VanillaEntityMobCategory::Monster)
    );
}

#[test]
fn category_filters_apply_enabled_spawn_flags() {
    let state = NaturalSpawnState::new(289);
    let categories = |friendly, hostile, persistent| {
        state.filtered_spawning_categories(friendly, hostile, persistent)
    };
    assert_eq!(
        categories(true, true, true),
        vec![
            VanillaEntityMobCategory::Monster,
            VanillaEntityMobCategory::Creature,
            VanillaEntityMobCategory::Ambient,
            VanillaEntityMobCategory::Axolotls,
            VanillaEntityMobCategory::UndergroundWaterCreature,
            VanillaEntityMobCategory::WaterCreature,
            VanillaEntityMobCategory::WaterAmbient
        ]
    );
    assert_eq!(
        categories(false, true, true),
        vec![VanillaEntityMobCategory::Monster]
    );
    assert_eq!(
        categories(true, false, true),
        vec![
            VanillaEntityMobCategory::Creature,
            VanillaEntityMobCategory::Ambient,
            VanillaEntityMobCategory::Axolotls,
            VanillaEntityMobCategory::UndergroundWaterCreature,
            VanillaEntityMobCategory::WaterCreature,
            VanillaEntityMobCategory::WaterAmbient
        ]
    );
    assert!(!state
        .filtered_spawning_categories(true, true, false)
        .contains(&VanillaEntityMobCategory::Creature));
}

#[test]
fn local_caps_allow_any_unsaturated_nearby_player_and_reject_no_players() {
    let first = player(1);
    let second = player(2);
    let mut state = NaturalSpawnState::new(289);
    for _ in 0..70 {
        state.observe_existing(existing(VanillaEntityMobCategory::Monster, &[first]));
    }
    assert!(!state.can_spawn_for_category_local(VanillaEntityMobCategory::Monster, &[]));
    assert!(!state.can_spawn_for_category_local(VanillaEntityMobCategory::Monster, &[first]));
    assert!(state.can_spawn_for_category_local(VanillaEntityMobCategory::Monster, &[first, second]));
    assert!(state.can_spawn_for_category_local(VanillaEntityMobCategory::Monster, &[second]));
}

#[test]
fn potential_uses_low_corner_inverse_distance_and_vanilla_zero_rules() {
    let mut potential = NaturalSpawnPotential::default();
    potential.add_charge(ORIGIN, 2.0);
    potential.add_charge(ORIGIN, 0.0);
    assert_eq!(potential.charge_count(), 1);
    assert_eq!(
        potential.energy_change(BlockPos { x: 3, y: 4, z: 0 }, 5.0),
        2.0
    );
    assert_eq!(potential.energy_change(ORIGIN, 0.0), 0.0);
    assert_eq!(potential.energy_change(ORIGIN, 1.0), f64::INFINITY);
}

#[test]
fn spawn_budget_and_published_mobs_update_potential_and_caps() {
    let nearby = [player(7)];
    let mut state = NaturalSpawnState::new(289);
    state.observe_existing(ExistingNaturalSpawnEntity {
        cost_charge: Some(1.0),
        ..existing(VanillaEntityMobCategory::Monster, &[])
    });
    let checked = BlockPos { x: 1, y: 0, z: 0 };
    assert!(state.can_spawn(
        checked,
        Some(NaturalSpawnCost {
            energy_budget: 2.0,
            charge: 2.0
        })
    ));
    assert!(!state.can_spawn(
        checked,
        Some(NaturalSpawnCost {
            energy_budget: 1.999,
            charge: 2.0
        })
    ));
    state.after_spawn(
        VanillaEntityMobCategory::Monster,
        checked,
        Some(NaturalSpawnCost {
            energy_budget: 0.0,
            charge: 2.0,
        }),
        &nearby,
    );
    assert_eq!(
        state
            .potential()
            .energy_change(BlockPos { x: 2, y: 0, z: 0 }, 1.0),
        2.5
    );
    state.after_spawn(
        VanillaEntityMobCategory::Creature,
        BlockPos { x: 3, y: 0, z: 0 },
        Some(NaturalSpawnCost {
            energy_budget: 0.0,
            charge: 4.0,
        }),
        &nearby,
    );
    assert_eq!(state.category_count(VanillaEntityMobCategory::Monster), 2);
    assert_eq!(state.category_count(VanillaEntityMobCategory::Creature), 1);
    assert_eq!(
        state.local_category_count(nearby[0], VanillaEntityMobCategory::Monster),
        1
    );
    assert_eq!(
        state.local_category_count(nearby[0], VanillaEntityMobCategory::Creature),
        1
    );
    assert_eq!(state.potential().charge_count(), 3);
}

#[test]
fn mobs_without_spawn_cost_do_not_increase_potential() {
    let mut state = NaturalSpawnState::new(289);
    assert!(state.can_spawn(ORIGIN, None));
    state.after_spawn(VanillaEntityMobCategory::Ambient, ORIGIN, None, &[]);
    assert_eq!(state.potential().charge_count(), 0);
}
