use super::*;

#[test]
fn the_level_cost_curve_matches_the_expected_branches() {
    let at = |level: i32| PlayerExperience::from_saved(0.0, level, 0).xp_needed_for_next_level();
    assert_eq!(at(0), 7);
    assert_eq!(at(14), 7 + 14 * 2);
    assert_eq!(at(15), 37);
    assert_eq!(at(29), 37 + 14 * 5);
    assert_eq!(at(30), 112);
    assert_eq!(at(31), 112 + 9);
}

/// `Player.giveExperiencePoints` (`Player.java:1500-1521`) over the first
/// levels: seven points is exactly one level from zero, and the progress it
/// leaves behind is the float division vanilla performs.
#[test]
fn awarding_points_climbs_the_levels_the_way_vanilla_does() {
    let mut experience = PlayerExperience::created();
    experience.give_experience_points(6);
    assert_eq!(experience.level(), 0);
    assert_eq!(experience.total(), 6);
    assert_eq!(experience.progress(), 6.0 / 7.0);

    experience.give_experience_points(1);
    assert_eq!(experience.level(), 1, "the seventh point levels up");
    assert_eq!(experience.total(), 7);
    assert_eq!(experience.progress(), 0.0);

    // Level one costs nine, so eight more leaves the bar just short.
    experience.give_experience_points(8);
    assert_eq!(experience.level(), 1);
    assert_eq!(experience.progress(), 8.0 / 9.0);
    assert_eq!(experience.total(), 15);
}

/// The negative arm (`:1505-1514`): a debit below zero empties the player
/// rather than going negative, which is `giveExperienceLevels`' own clamp.
#[test]
fn a_debit_past_zero_empties_the_bar_instead_of_going_negative() {
    let mut experience = PlayerExperience::created();
    experience.give_experience_points(7);
    experience.give_experience_points(-100);
    assert_eq!(experience.level(), 0);
    assert_eq!(experience.progress(), 0.0);
    assert_eq!(experience.total(), 0);
}

fn lcg_next_int(seed: &mut u64, bound: i32) -> i32 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 33) % (bound as u64)) as i32
}

#[test]
fn the_experience_split_matches_the_ladder_thresholds() {
    let thresholds = [2477, 1237, 617, 307, 149, 73, 37, 17, 7, 3];
    for threshold in thresholds {
        assert_eq!(experience_value(threshold), threshold, "at {threshold}");
        assert_ne!(
            experience_value(threshold - 1),
            threshold,
            "one below {threshold} falls to the rung under it"
        );
    }
    assert_eq!(experience_value(2), 1);
    assert_eq!(experience_value(1), 1);
}

#[test]
fn spawner_magic_count_skips_enchantment_and_samples_expected_range() {
    assert!(!BreakExperience::SpawnerMagicCount.is_enchantment_modified());
    assert!(BreakExperience::Constant(5).is_enchantment_modified());
    let mut seed = 1234_u64;
    for _ in 0..64 {
        let sampled =
            BreakExperience::SpawnerMagicCount.sample(|bound| lcg_next_int(&mut seed, bound));
        assert!((15..=43).contains(&sampled), "{sampled}");
    }
}

#[test]
fn sampled_providers_stay_within_their_declared_ranges() {
    let mut seed = 42_u64;
    for _ in 0..32 {
        assert!((3..=7).contains(
            &BreakExperience::Uniform(3, 7).sample(|bound| lcg_next_int(&mut seed, bound))
        ));
    }
    assert_eq!(
        BreakExperience::Constant(0).sample(|_| panic!("Constant draws nothing")),
        0
    );
}

#[test]
fn extreme_experience_levels_saturate_without_overflow_or_infinite_loop() {
    let mut xp = PlayerExperience::from_saved(0.99, i32::MAX, i32::MAX);
    assert_eq!(xp.xp_needed_for_next_level(), i32::MAX);
    xp.give_experience_points(i32::MAX);
    assert_eq!(xp.level(), i32::MAX);
    assert!(xp.progress().is_finite() && xp.progress() < 1.0);

    let neg = PlayerExperience::from_saved(0.0, -100, 0);
    assert_eq!(neg.xp_needed_for_next_level(), 7);
}
