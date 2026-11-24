//! The three `canSurvive` bodies that read `getRawBrightness`, over a shape
//! view that either carries a light engine or does not.

use crate::block_getter::BlockGetter;

use std::collections::HashMap;

use bbb_protocol::packets::BlockPos;

use super::{default_state, state_of};
use crate::block_behavior::can_survive_at;

struct LightLevel {
    states: HashMap<(i32, i32, i32), i32>,
    raw_brightness: Option<i32>,
}

fn key(pos: BlockPos) -> (i32, i32, i32) {
    (pos.x, pos.y, pos.z)
}

impl BlockGetter for LightLevel {
    fn block_state(&self, pos: BlockPos) -> i32 {
        self.states
            .get(&key(pos))
            .copied()
            .unwrap_or_else(|| default_state("minecraft:air"))
    }

    fn raw_brightness(&self, _pos: BlockPos, _sky_darkening: i32) -> Option<i32> {
        self.raw_brightness
    }
}

const ORIGIN: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
const BELOW: BlockPos = BlockPos { x: 0, y: 63, z: 0 };

fn level(below: &str, raw_brightness: Option<i32>) -> LightLevel {
    LightLevel {
        states: HashMap::from([(key(BELOW), default_state(below))]),
        raw_brightness,
    }
}

#[test]
fn a_crop_wants_eight_and_farmland() {
    let wheat = default_state("minecraft:wheat");
    assert!(can_survive_at(
        wheat,
        ORIGIN,
        &level("minecraft:farmland", Some(8))
    ));
    assert!(!can_survive_at(
        wheat,
        ORIGIN,
        &level("minecraft:farmland", Some(7))
    ));
    // Light first, then `mayPlaceOn` (`CropBlock.java:145-147`, `:49-51`).
    assert!(!can_survive_at(
        wheat,
        ORIGIN,
        &level("minecraft:dirt", Some(15))
    ));
}

#[test]
fn a_mushroom_reads_its_tag_before_the_light() {
    {
        let mushroom = "minecraft:brown_mushroom";
        let state = default_state(mushroom);
        // `#minecraft:overrides_mushroom_light_requirement` answers first, so
        // full daylight over mycelium still keeps the mushroom
        // (`MushroomBlock.java:86`).
        assert!(
            can_survive_at(state, ORIGIN, &level("minecraft:mycelium", Some(15))),
            "{mushroom}"
        );
        assert!(
            can_survive_at(state, ORIGIN, &level("minecraft:stone", Some(12))),
            "{mushroom}"
        );
        assert!(
            !can_survive_at(state, ORIGIN, &level("minecraft:stone", Some(13))),
            "{mushroom}"
        );
        // Dark enough, but `mayPlaceOn` wants a solid render below.
        assert!(
            !can_survive_at(state, ORIGIN, &level("minecraft:glass", Some(0))),
            "{mushroom}"
        );
    }
}

#[test]
fn only_the_lower_pitcher_crop_half_reads_the_light() {
    let lower = state_of("minecraft:pitcher_crop", &[("age", "3"), ("half", "lower")]);
    let upper = state_of("minecraft:pitcher_crop", &[("age", "3"), ("half", "upper")]);
    assert!(can_survive_at(
        lower,
        ORIGIN,
        &level("minecraft:farmland", Some(8))
    ));
    assert!(!can_survive_at(
        lower,
        ORIGIN,
        &level("minecraft:farmland", Some(7))
    ));

    // `DoublePlantBlock.canSurvive`'s upper arm asks only for its own lower
    // half (`DoublePlantBlock.java:77-84`), so a view with no light answers it.
    let mut standing = LightLevel {
        states: HashMap::from([(key(BELOW), lower)]),
        raw_brightness: None,
    };
    assert!(can_survive_at(upper, ORIGIN, &standing));
    standing
        .states
        .insert(key(BELOW), default_state("minecraft:farmland"));
    assert!(!can_survive_at(upper, ORIGIN, &standing));
}
