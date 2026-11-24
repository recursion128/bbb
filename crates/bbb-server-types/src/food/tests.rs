use super::*;

use crate::player_damage::player_max_health;

#[test]
fn a_created_food_data_is_full_and_unsaturated_by_exhaustion() {
    let food = FoodData::created();
    assert_eq!(food.food_level(), 20);
    assert_eq!(food.saturation_level(), 5.0);
    assert_eq!(food.exhaustion_level(), 0.0);
    assert_eq!(food.tick_timer(), 0);
    assert!(!food.needs_food());
    assert!(food.has_enough_food());
}

/// Four points of exhaustion buy one point of saturation, and the cascade only
/// runs when the level is *strictly* above four (`FoodData.java:35`).
#[test]
fn the_exhaustion_cascade_spends_saturation_first() {
    let mut food = FoodData::created();
    food.add_exhaustion(4.0);
    assert_eq!(
        food.tick(
            Difficulty::Normal,
            true,
            player_max_health(),
            player_max_health()
        ),
        FoodTickEffect::Nothing
    );
    assert_eq!(food.exhaustion_level(), 4.0, "exactly four does not drop");
    assert_eq!(food.saturation_level(), 5.0);

    food.add_exhaustion(0.5);
    assert_eq!(
        food.tick(
            Difficulty::Normal,
            true,
            player_max_health(),
            player_max_health()
        ),
        FoodTickEffect::Nothing
    );
    assert_eq!(food.exhaustion_level(), 0.5);
    assert_eq!(food.saturation_level(), 4.0);
    assert_eq!(
        food.food_level(),
        20,
        "food only falls once saturation is 0"
    );
}

/// With no saturation left the cascade takes a point of food -- unless the
/// difficulty is PEACEFUL, which is the only branch difficulty takes here
/// (`FoodData.java:39`).
#[test]
fn the_exhaustion_cascade_spares_food_on_peaceful() {
    for (difficulty, expected_food) in [
        (Difficulty::Peaceful, 20),
        (Difficulty::Easy, 19),
        (Difficulty::Normal, 19),
        (Difficulty::Hard, 19),
    ] {
        let mut food = FoodData::from_saved(20, 0, 0.0, 5.0);
        food.tick(difficulty, false, player_max_health(), player_max_health());
        assert_eq!(
            food.food_level(),
            expected_food,
            "{difficulty:?} drained the wrong amount"
        );
        assert_eq!(food.exhaustion_level(), 1.0);
    }
}

/// The fast branch: full food, some saturation, a hurt player and the game rule
/// on. Ten ticks buy `min(saturation, 6) / 6` health and the same number of
/// exhaustion points (`FoodData.java:45-52`).
#[test]
fn saturated_regeneration_heals_every_ten_ticks() {
    let mut food = FoodData::from_saved(20, 0, 5.0, 0.0);
    let health = player_max_health() - 4.0;
    for tick in 1..SATURATED_REGEN_TICKS {
        assert_eq!(
            food.tick(Difficulty::Normal, true, health, player_max_health()),
            FoodTickEffect::Nothing,
            "tick {tick} healed early"
        );
    }
    assert_eq!(
        food.tick(Difficulty::Normal, true, health, player_max_health()),
        FoodTickEffect::Heal(5.0 / 6.0)
    );
    assert_eq!(food.tick_timer(), 0);
    assert_eq!(
        food.exhaustion_level(),
        5.0,
        "the heal charges its own cost"
    );
}

/// The same branch with more than six saturation spends only six of it.
#[test]
fn saturated_regeneration_spends_at_most_six_saturation() {
    let mut food = FoodData::from_saved(20, SATURATED_REGEN_TICKS - 1, 20.0, 0.0);
    assert_eq!(
        food.tick(Difficulty::Normal, true, 1.0, player_max_health()),
        FoodTickEffect::Heal(1.0)
    );
    assert_eq!(food.exhaustion_level(), 6.0);
}

/// The slow branch needs eighteen food rather than twenty and waits eighty
/// ticks for one point of health (`FoodData.java:53-59`).
#[test]
fn slow_regeneration_heals_every_eighty_ticks() {
    let mut food = FoodData::from_saved(18, SLOW_REGEN_TICKS - 1, 0.0, 0.0);
    assert_eq!(
        food.tick(Difficulty::Normal, true, 1.0, player_max_health()),
        FoodTickEffect::Heal(1.0)
    );
    assert_eq!(food.exhaustion_level(), 6.0);
}

/// The game rule gates both regeneration branches and nothing else: with it off
/// a hurt, well-fed player simply keeps its timer at zero.
#[test]
fn natural_regeneration_off_stops_both_heal_branches() {
    let mut food = FoodData::from_saved(20, 0, 5.0, 0.0);
    for _ in 0..SLOW_REGEN_TICKS * 2 {
        assert_eq!(
            food.tick(Difficulty::Normal, false, 1.0, player_max_health()),
            FoodTickEffect::Nothing
        );
    }
    assert_eq!(food.tick_timer(), 0);
}

/// A player at full health takes neither branch, because `Player.isHurt` is
/// false there (`Player.java:1586`).
#[test]
fn a_full_health_player_never_regenerates() {
    let mut food = FoodData::from_saved(20, 0, 5.0, 0.0);
    for _ in 0..SLOW_REGEN_TICKS * 2 {
        assert_eq!(
            food.tick(
                Difficulty::Normal,
                true,
                player_max_health(),
                player_max_health()
            ),
            FoodTickEffect::Nothing
        );
    }
    assert_eq!(food.tick_timer(), 0);
}

/// Starvation's three floors (`FoodData.java:63`). PEACEFUL and EASY stop at
/// five hearts, NORMAL at half a heart, HARD kills.
#[test]
fn starvation_floors_follow_the_difficulty() {
    let cases = [
        (Difficulty::Peaceful, 10.5, true),
        (Difficulty::Peaceful, 10.0, false),
        (Difficulty::Easy, 10.5, true),
        (Difficulty::Easy, 10.0, false),
        (Difficulty::Normal, 10.0, true),
        (Difficulty::Normal, 1.5, true),
        (Difficulty::Normal, 1.0, false),
        (Difficulty::Hard, 1.0, true),
        (Difficulty::Hard, 0.5, true),
    ];
    for (difficulty, health, starves) in cases {
        let mut food = FoodData::from_saved(0, STARVE_TICKS - 1, 0.0, 0.0);
        let effect = food.tick(difficulty, true, health, player_max_health());
        let expected = if starves {
            FoodTickEffect::Starve
        } else {
            FoodTickEffect::Nothing
        };
        assert_eq!(effect, expected, "{difficulty:?} at {health} health");
        assert_eq!(food.tick_timer(), 0, "the timer resets either way");
    }
}

/// A player who is neither regenerating nor starving has its timer reset every
/// tick, so a partial wait never carries into a later branch
/// (`FoodData.java:69-71`).
#[test]
fn the_fallthrough_arm_resets_the_timer() {
    let mut food = FoodData::from_saved(10, 40, 0.0, 0.0);
    assert_eq!(
        food.tick(Difficulty::Normal, true, 1.0, player_max_health()),
        FoodTickEffect::Nothing
    );
    assert_eq!(food.tick_timer(), 0);
}

/// `causeFoodExhaustion` refuses a player whose abilities say invulnerable,
/// which is every creative and spectator session.
#[test]
fn cause_food_exhaustion_spares_the_invulnerable() {
    let mut food = FoodData::created();
    cause_food_exhaustion(&mut food, true, EXHAUSTION_MINE);
    assert_eq!(food.exhaustion_level(), 0.0);
    cause_food_exhaustion(&mut food, false, EXHAUSTION_MINE);
    assert_eq!(food.exhaustion_level(), EXHAUSTION_MINE);
}

/// `addExhaustion` saturates at forty rather than growing without bound.
#[test]
fn exhaustion_saturates_at_forty() {
    let mut food = FoodData::created();
    food.add_exhaustion(100.0);
    assert_eq!(food.exhaustion_level(), MAX_EXHAUSTION_LEVEL);
}

/// `eat` multiplies the modifier by twice the nutrition and clamps the
/// saturation to the new food level, not the old one.
#[test]
fn eating_clamps_saturation_to_the_new_food_level() {
    let mut food = FoodData::from_saved(0, 0, 0.0, 0.0);
    // Cake: two nutrition at a 0.1 modifier (`CakeBlock.java:100`).
    food.eat(2, 0.1);
    assert_eq!(food.food_level(), 2);
    assert_eq!(food.saturation_level(), saturation_by_modifier(2, 0.1));

    let mut food = FoodData::from_saved(0, 0, 0.0, 0.0);
    food.eat(1, 20.0);
    assert_eq!(food.food_level(), 1);
    assert_eq!(
        food.saturation_level(),
        1.0,
        "clamped to the new food level"
    );
}
