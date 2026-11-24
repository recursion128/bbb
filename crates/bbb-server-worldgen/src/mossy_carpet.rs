use bbb_protocol::block_state_types::{
    vanilla_block_state_identity, vanilla_block_state_support_facts,
};
use bbb_protocol::packets::{BlockPos, Direction};

use super::{
    block_is, block_set_property, catalog_default_state, is_empty_block, offset_block_pos,
    WorldGenLevel, HORIZONTAL_FACES,
};

const PALE_MOSS_CARPET: &str = "minecraft:pale_moss_carpet";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WallSide {
    None,
    Low,
    Tall,
}

impl WallSide {
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Tall => "tall",
        }
    }
}

/// `MossyCarpetBlock.canSurvive` (`MossyCarpetBlock.java:104-107`).
pub fn can_survive<L: WorldGenLevel>(level: &L, pos: BlockPos, state: i32) -> Option<bool> {
    let below = level.get_block(offset_block_pos(pos, Direction::Down));
    if bottom(state)? {
        Some(!is_empty_block(below))
    } else {
        Some(block_is(below, PALE_MOSS_CARPET) && bottom(below)?)
    }
}

/// `MossyCarpetBlock.placeAt` (`MossyCarpetBlock.java:166-176`).
pub fn place_at<L: WorldGenLevel>(level: &mut L, pos: BlockPos, update_flags: i32) -> Option<()> {
    let simple = catalog_default_state(PALE_MOSS_CARPET);
    let adjusted = updated_state(simple, level, pos, true)?;
    let _ = level.set_block(pos, adjusted, update_flags);
    if let Some(topper) = create_topper_with_side_chance(level, pos)? {
        let above = offset_block_pos(pos, Direction::Up);
        let _ = level.set_block(above, topper, update_flags);
        let updated_bottom = updated_state(adjusted, level, pos, true)?;
        let _ = level.set_block(pos, updated_bottom, update_flags);
    }
    Some(())
}

/// `MossyCarpetBlock.getUpdatedState` (`MossyCarpetBlock.java:127-159`).
fn updated_state<L: WorldGenLevel>(
    mut state: i32,
    level: &L,
    pos: BlockPos,
    mut create_sides: bool,
) -> Option<i32> {
    let is_bottom = bottom(state)?;
    create_sides |= is_bottom;
    let mut above_state = None;
    let mut below_state = None;

    for direction in HORIZONTAL_FACES {
        let property = side_property(direction)?;
        let mut side = if can_support_at_face(level, pos, direction) {
            if create_sides {
                WallSide::Low
            } else {
                wall_side(state, property)?
            }
        } else {
            WallSide::None
        };
        if side == WallSide::Low {
            let above = *above_state
                .get_or_insert_with(|| level.get_block(offset_block_pos(pos, Direction::Up)));
            if block_is(above, PALE_MOSS_CARPET)
                && wall_side(above, property)? != WallSide::None
                && !bottom(above)?
            {
                side = WallSide::Tall;
            }
            if !is_bottom {
                let below = *below_state
                    .get_or_insert_with(|| level.get_block(offset_block_pos(pos, Direction::Down)));
                if block_is(below, PALE_MOSS_CARPET)
                    && wall_side(below, property)? == WallSide::None
                {
                    side = WallSide::None;
                }
            }
        }
        state = block_set_property(state, property, side.as_str());
    }
    Some(state)
}

/// `MossyCarpetBlock.createTopperWithSideChance`
/// (`MossyCarpetBlock.java:189-208`).
fn create_topper_with_side_chance<L: WorldGenLevel>(
    level: &mut L,
    pos: BlockPos,
) -> Option<Option<i32>> {
    let above = offset_block_pos(pos, Direction::Up);
    let previous = level.get_block(above);
    let is_mossy = block_is(previous, PALE_MOSS_CARPET);
    let eligible = if is_mossy {
        !bottom(previous)?
    } else {
        vanilla_block_state_support_facts(previous)?.replaceable()
    };
    if !eligible {
        return Some(None);
    }

    let no_base = block_set_property(catalog_default_state(PALE_MOSS_CARPET), "bottom", "false");
    let mut state = updated_state(no_base, level, above, true)?;
    for direction in HORIZONTAL_FACES {
        let property = side_property(direction)?;
        if wall_side(state, property)? != WallSide::None && !level.next_level_random_boolean() {
            state = block_set_property(state, property, WallSide::None.as_str());
        }
    }
    Some((has_faces(state)? && state != previous).then_some(state))
}

/// `MossyCarpetBlock.hasFaces` (`MossyCarpetBlock.java:109-121`).
fn has_faces(state: i32) -> Option<bool> {
    if bottom(state)? {
        return Some(true);
    }
    for direction in HORIZONTAL_FACES {
        if wall_side(state, side_property(direction)?)? != WallSide::None {
            return Some(true);
        }
    }
    Some(false)
}

/// `MossyCarpetBlock.canSupportAtFace` (`MossyCarpetBlock.java:123-125`).
fn can_support_at_face<L: WorldGenLevel>(level: &L, pos: BlockPos, direction: Direction) -> bool {
    if direction == Direction::Up {
        return false;
    }
    let neighbour_pos = offset_block_pos(pos, direction);
    super::multiface::can_attach_to_at(
        level,
        neighbour_pos,
        level.get_block(neighbour_pos),
        direction,
    )
}

fn bottom(state: i32) -> Option<bool> {
    match property(state, "bottom")? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn wall_side(state: i32, property: &str) -> Option<WallSide> {
    match self::property(state, property)? {
        "none" => Some(WallSide::None),
        "low" => Some(WallSide::Low),
        "tall" => Some(WallSide::Tall),
        _ => None,
    }
}

fn property(state: i32, name: &str) -> Option<&'static str> {
    vanilla_block_state_identity(state)
        .and_then(|identity| identity.properties.get(name))
        .map(String::as_str)
}

fn side_property(direction: Direction) -> Option<&'static str> {
    match direction {
        Direction::North => Some("north"),
        Direction::East => Some("east"),
        Direction::South => Some("south"),
        Direction::West => Some("west"),
        Direction::Down | Direction::Up => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, VecDeque};

    use super::*;

    struct TestLevel {
        cells: BTreeMap<(i32, i32, i32), i32>,
        sets: Vec<(BlockPos, i32, i32)>,
        booleans: VecDeque<bool>,
        apply_writes: bool,
        set_result: bool,
    }

    impl TestLevel {
        fn new() -> Self {
            Self {
                cells: BTreeMap::new(),
                sets: Vec::new(),
                booleans: VecDeque::new(),
                apply_writes: true,
                set_result: true,
            }
        }

        fn put(&mut self, pos: BlockPos, state: i32) {
            self.cells.insert(key(pos), state);
        }

        fn with_booleans(mut self, values: impl IntoIterator<Item = bool>) -> Self {
            self.booleans.extend(values);
            self
        }
    }

    impl WorldGenLevel for TestLevel {
        fn ensure_can_write(&self, _origin: BlockPos) -> bool {
            true
        }

        fn get_block(&self, pos: BlockPos) -> i32 {
            self.cells.get(&key(pos)).copied().unwrap_or_else(air)
        }

        fn set_block(&mut self, pos: BlockPos, state: i32, update_flags: i32) -> bool {
            self.sets.push((pos, state, update_flags));
            if self.apply_writes {
                self.cells.insert(key(pos), state);
            }
            self.set_result
        }

        fn get_min_y(&self) -> i32 {
            -64
        }

        fn get_max_y(&self) -> i32 {
            319
        }

        fn get_sea_level(&self) -> i32 {
            63
        }

        fn schedule_fluid_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn schedule_block_tick(&mut self, _pos: BlockPos, _delay: i32) {}

        fn mark_pos_for_postprocessing(&mut self, _pos: BlockPos) {}

        fn next_level_random_boolean(&mut self) -> bool {
            self.booleans.pop_front().expect("scripted level random")
        }
    }

    fn key(pos: BlockPos) -> (i32, i32, i32) {
        (pos.x, pos.y, pos.z)
    }

    fn pos(x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos { x, y, z }
    }

    fn state(name: &'static str) -> i32 {
        catalog_default_state(name)
    }

    fn air() -> i32 {
        state("minecraft:air")
    }

    fn pale(bottom_value: bool, sides: [WallSide; 4]) -> i32 {
        let mut state = state(PALE_MOSS_CARPET);
        state = block_set_property(state, "bottom", if bottom_value { "true" } else { "false" });
        for (direction, side) in HORIZONTAL_FACES.into_iter().zip(sides) {
            state = block_set_property(state, side_property(direction).unwrap(), side.as_str());
        }
        state
    }

    fn support(level: &mut TestLevel, origin: BlockPos, direction: Direction, height: i32) {
        let base = BlockPos {
            x: origin.x,
            y: origin.y + height,
            z: origin.z,
        };
        level.put(offset_block_pos(base, direction), state("minecraft:stone"));
    }

    fn prepare_survival(level: &mut TestLevel, origin: BlockPos) {
        level.put(
            offset_block_pos(origin, Direction::Down),
            state("minecraft:stone"),
        );
    }

    #[test]
    fn bottom_and_topper_survival_follow_the_bottom_property() {
        let origin = pos(0, 64, 0);
        let mut level = TestLevel::new();
        let bottom_state = pale(true, [WallSide::None; 4]);
        let topper_state = pale(
            false,
            [
                WallSide::Low,
                WallSide::None,
                WallSide::None,
                WallSide::None,
            ],
        );

        assert!(!can_survive(&level, origin, bottom_state).unwrap());
        level.put(
            offset_block_pos(origin, Direction::Down),
            state("minecraft:water"),
        );
        assert!(can_survive(&level, origin, bottom_state).unwrap());

        level.put(offset_block_pos(origin, Direction::Down), bottom_state);
        assert!(can_survive(&level, origin, topper_state).unwrap());
        level.put(
            offset_block_pos(origin, Direction::Down),
            pale(
                false,
                [
                    WallSide::Low,
                    WallSide::None,
                    WallSide::None,
                    WallSide::None,
                ],
            ),
        );
        assert!(!can_survive(&level, origin, topper_state).unwrap());
        level.put(
            offset_block_pos(origin, Direction::Down),
            state("minecraft:stone"),
        );
        assert!(!can_survive(&level, origin, topper_state).unwrap());
    }

    #[test]
    fn no_support_writes_only_the_bottom() {
        let origin = pos(0, 64, 0);
        let mut level = TestLevel::new();
        prepare_survival(&mut level, origin);

        place_at(&mut level, origin, 37).unwrap();

        assert_eq!(level.sets, [(origin, pale(true, [WallSide::None; 4]), 37)]);
    }

    #[test]
    fn four_supported_faces_grow_a_topper_and_rewrite_the_bottom() {
        let origin = pos(0, 64, 0);
        let mut level = TestLevel::new().with_booleans([true; 4]);
        prepare_survival(&mut level, origin);
        for direction in HORIZONTAL_FACES {
            support(&mut level, origin, direction, 0);
            support(&mut level, origin, direction, 1);
        }

        place_at(&mut level, origin, 2).unwrap();

        assert_eq!(
            level.sets,
            [
                (origin, pale(true, [WallSide::Low; 4]), 2),
                (
                    offset_block_pos(origin, Direction::Up),
                    pale(false, [WallSide::Low; 4]),
                    2,
                ),
                (origin, pale(true, [WallSide::Tall; 4]), 2),
            ]
        );
    }

    #[test]
    fn only_faces_supported_at_both_levels_grow_on_the_topper() {
        let origin = pos(0, 64, 0);
        let mut level = TestLevel::new().with_booleans([true, true]);
        prepare_survival(&mut level, origin);
        for direction in [Direction::North, Direction::East] {
            support(&mut level, origin, direction, 0);
            support(&mut level, origin, direction, 1);
        }
        support(&mut level, origin, Direction::South, 1);
        support(&mut level, origin, Direction::West, 0);

        place_at(&mut level, origin, 2).unwrap();

        assert_eq!(
            level.sets[1].1,
            pale(
                false,
                [WallSide::Low, WallSide::Low, WallSide::None, WallSide::None]
            )
        );
        assert_eq!(
            level.sets[2].1,
            pale(
                true,
                [
                    WallSide::Tall,
                    WallSide::Tall,
                    WallSide::None,
                    WallSide::Low
                ]
            )
        );
    }

    #[test]
    fn stone_and_bottom_carpet_above_reject_a_topper() {
        let origin = pos(0, 64, 0);
        for above_state in [state("minecraft:stone"), pale(true, [WallSide::Low; 4])] {
            let mut level = TestLevel::new();
            prepare_survival(&mut level, origin);
            level.put(offset_block_pos(origin, Direction::Up), above_state);
            for direction in HORIZONTAL_FACES {
                support(&mut level, origin, direction, 0);
                support(&mut level, origin, direction, 1);
            }

            place_at(&mut level, origin, 2).unwrap();

            assert_eq!(level.sets.len(), 1);
        }
    }
}
