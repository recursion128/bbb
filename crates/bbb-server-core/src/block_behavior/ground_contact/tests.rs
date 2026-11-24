use super::*;
use crate::fluid::with_property;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_TYPES};

pub(super) fn body() -> GroundBody {
    GroundBody {
        living: true,
        player: false,
        zombie: false,
        turtle_or_bat: false,
        careful: false,
        mob_griefing: Some(true),
        width: 0.6,
        height: 1.8,
    }
}

/// A level random whose next draws satisfy `first_draws`.
pub(super) fn random_where(first_draws: impl Fn(&GameRandom) -> bool) -> GameRandom {
    (0..)
        .find(|seed| first_draws(&GameRandom::with_seed(*seed)))
        .map(GameRandom::with_seed)
        .unwrap()
}

fn state(block: i32) -> i32 {
    vanilla_block_default_state_id(block).unwrap()
}

fn land(state: i32, distance: f64, body: GroundBody) -> FallOn {
    fall_on(state, distance, body, &GameRandom::with_seed(0)).unwrap()
}

fn fall(distance: f64, modifier: f32, damage_type: &'static str) -> Option<FallDamage> {
    Some(FallDamage {
        distance,
        modifier,
        damage_type,
    })
}

#[test]
fn ground_contact_default_and_hay_damage_modifiers() {
    for (block, modifier) in [
        (VANILLA_BLOCK_STONE_ID, 1.0),
        (VANILLA_BLOCK_HAY_BLOCK_ID, 0.2),
    ] {
        assert_eq!(
            land(state(block), 12.25, body()),
            FallOn {
                block: FallOnBlock::None,
                damage: fall(12.25, modifier, "minecraft:fall"),
                damaged_sound: None,
            }
        );
    }
}

#[test]
fn ground_contact_bed_halves_fall_distance() {
    let bed = VANILLA_BLOCK_TYPES
        .iter()
        .find(|b| b.use_without_item_override == Some(VanillaBlockUseWithoutItemOverride::BedBlock))
        .unwrap();
    assert_eq!(
        land(state(bed.id), 7.5, body()).damage,
        fall(3.75, 1.0, "minecraft:fall")
    );
}

#[test]
fn ground_contact_slime_suppression_skips_damage_entirely() {
    for careful in [false, true] {
        assert_eq!(
            land(
                state(VANILLA_BLOCK_SLIME_BLOCK_ID),
                30.0,
                GroundBody { careful, ..body() }
            )
            .damage,
            (!careful)
                .then(|| fall(30.0, 0.0, "minecraft:fall"))
                .flatten()
        );
    }
}

#[test]
fn ground_contact_powder_snow_uses_living_sound_thresholds_without_damage() {
    for living in [false, true] {
        for (distance, sound) in [
            (3.999, None),
            (4.0, Some(false)),
            (6.999, Some(false)),
            (7.0, Some(true)),
        ] {
            let landing = land(
                state(VANILLA_BLOCK_POWDER_SNOW_ID),
                distance,
                GroundBody { living, ..body() },
            );
            assert_eq!(landing.damage, None);
            assert_eq!(
                landing.block,
                match sound.filter(|_| living) {
                    Some(big) => FallOnBlock::FallSound { big },
                    None => FallOnBlock::None,
                }
            );
        }
    }
}

#[test]
fn ground_contact_dripstone_only_upward_unmerged_tip_changes_damage_source() {
    for (direction, thickness) in [
        ("up", "tip"),
        ("up", "tip_merge"),
        ("up", "base"),
        ("down", "tip"),
    ] {
        let state = with_property(
            state(VANILLA_BLOCK_POINTED_DRIPSTONE_ID),
            "vertical_direction",
            direction,
        )
        .unwrap();
        let state = with_property(state, "thickness", thickness).unwrap();
        let sharp = direction == "up" && thickness == "tip";
        assert_eq!(
            land(state, 8.0, body()).damage,
            if sharp {
                fall(10.5, 2.0, "minecraft:stalagmite")
            } else {
                fall(8.0, 1.0, "minecraft:fall")
            }
        );
    }
}

#[test]
fn ground_contact_honey_reduces_damage_and_plays_fall_sound_on_success() {
    assert_eq!(
        land(state(VANILLA_BLOCK_HONEY_BLOCK_ID), 10.0, body()),
        FallOn {
            block: FallOnBlock::HoneySlide,
            damage: fall(10.0, 0.2, "minecraft:fall"),
            damaged_sound: Some(("minecraft:block.honey_block.fall", 0.5, 0.75)),
        }
    );
}

fn tramples(distance: f64, body: GroundBody, random: &GameRandom) -> bool {
    let landing = fall_on(state(VANILLA_BLOCK_FARMLAND_ID), distance, body, random).unwrap();
    assert_eq!(landing.damage, fall(distance, 1.0, "minecraft:fall"));
    landing.block == FallOnBlock::TrampleFarmland
}

#[test]
fn ground_contact_farmland_needs_living_body_and_enough_fall() {
    let low = || random_where(|random| random.next_float() < 0.25);
    let high = || random_where(|random| random.next_float() >= 0.5);
    assert!(tramples(1.0, body(), &low()));
    assert!(!tramples(0.5, body(), &low()));
    assert!(!tramples(1.0, body(), &high()));
    assert!(!tramples(
        10.0,
        GroundBody {
            living: false,
            ..body()
        },
        &low()
    ));
}

#[test]
fn ground_contact_farmland_player_bypasses_griefing_rule_and_size_is_strict_f32() {
    for (height, expected) in [
        (0.512_f32, false),
        (f32::from_bits(0.512_f32.to_bits() + 1), true),
    ] {
        let player = GroundBody {
            player: true,
            mob_griefing: Some(false),
            width: 1.0,
            height,
            ..body()
        };
        assert_eq!(tramples(10.0, player, &GameRandom::with_seed(0)), expected);
    }
}

#[test]
fn ground_contact_farmland_mob_griefing_off_prevents_mob_trample() {
    let body = GroundBody {
        mob_griefing: Some(false),
        ..body()
    };
    assert!(!tramples(10.0, body, &GameRandom::with_seed(0)));
}

fn eggs_broken(body: GroundBody, random: &GameRandom) -> bool {
    let landing = fall_on(state(VANILLA_BLOCK_TURTLE_EGG_ID), 10.0, body, random).unwrap();
    assert_eq!(landing.damage, fall(10.0, 1.0, "minecraft:fall"));
    landing.block == FallOnBlock::DecreaseTurtleEggs
}

#[test]
fn ground_contact_turtle_egg_excluded_bodies_never_break_eggs_but_still_take_damage() {
    for body in [
        GroundBody {
            zombie: true,
            ..body()
        },
        GroundBody {
            turtle_or_bat: true,
            ..body()
        },
        GroundBody {
            living: false,
            ..body()
        },
        GroundBody {
            mob_griefing: Some(false),
            ..body()
        },
    ] {
        assert!(!eggs_broken(
            body,
            &random_where(|random| random.next_int(3) == 0)
        ));
    }
}

#[test]
fn ground_contact_turtle_egg_breaks_on_one_in_three_roll() {
    let player = GroundBody {
        player: true,
        ..body()
    };
    assert!(eggs_broken(
        player,
        &random_where(|random| random.next_int(3) == 0)
    ));
    assert!(!eggs_broken(
        player,
        &random_where(|random| random.next_int(3) != 0)
    ));
}
