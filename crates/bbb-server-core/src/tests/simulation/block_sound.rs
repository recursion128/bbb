//! The sound `SculkBlock.attemptUseCharge` plays for the growth block it
//! placed, on a real level.

use super::*;

use crate::random::GameRandom;
use bbb_server_types::sculk_cursor::SculkCursorFacts;
use bbb_server_worldgen::sculk_spreader::SCULK_BLOCK_SPREAD_SOUND;

use super::helpers::sculk_cursor as cursor;

const DIM: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const CATALYST: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
/// `SculkBlock.maxGrowthRateRadius`'s own radius is irrelevant here; the
/// cursor's cell, the growth's cell and the session are what the sound is
/// measured against.
const CURSOR: BlockPos = BlockPos { x: 10, y: 64, z: 0 };

fn catalyst_session_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    helpers::catalyst_session_world(CATALYST)
}

fn charge_spreader(world: &mut ServerWorld, facts: &[SculkCursorFacts]) {
    helpers::charge_spreader(world, CATALYST, facts);
}

/// The seed whose draws reach `SculkBlock.attemptUseCharge`'s placement branch
/// with the wanted growth state: `nextInt(chargeDecayRate)` is zero
/// the growth cost is paid and
/// `getRandomGrowthState`'s `nextInt(11)` picks the shrieker or the sensor
fn growth_seed(wants_shrieker: bool) -> i64 {
    (0_i64..)
        .find(|seed| {
            let draws = GameRandom::with_seed(*seed);
            if draws.next_int(10) != 0 {
                return false;
            }
            if draws.next_int(10) >= 1000 {
                return false;
            }
            (draws.next_int(11) == 0) == wants_shrieker
        })
        .expect("some seed reaches the growth branch with the wanted state")
}

/// `SculkBlock`: the growth the cursor draws plays the *placed state's*
/// own place sound, at the cursor's cell rather than the growth's, at volume
/// and pitch one, and after the write that put the block there.
///
/// The two seeds below draw the two states `getRandomGrowthState` can build, so
/// the sound that leaves is the one the drawn state answers: a hardcoded id
/// could not follow the draw.
#[test]
fn a_growth_plays_the_place_sound_of_the_state_it_drew() {
    for (wants_shrieker, expected) in [
        (false, "minecraft:block.sculk_sensor.place"),
        (true, "minecraft:block.sculk_shrieker.place"),
    ] {
        let seed = growth_seed(wants_shrieker);
        let draws = GameRandom::with_seed(seed);
        assert_eq!(draws.next_int(10), 0);
        assert!(draws.next_int(10) < 1000);
        assert_eq!(draws.next_int(11) == 0, wants_shrieker);

        let (sender, receiver, mut world) = catalyst_session_world();
        assert!(world.set_block(
            DIM,
            CURSOR,
            block_state_id("minecraft:sculk"),
            SET_BLOCK_UPDATE_ALL
        ));
        charge_spreader(&mut world, &[cursor(CURSOR, 1000, 1, 0)]);
        world.tick_sculk_catalyst_seeded_for_test(&DIM, CATALYST, seed);
        let growth = BlockPos {
            y: CURSOR.y + 1,
            ..CURSOR
        };
        // The write is what the sound follows (`:45` then `:46`).
        assert_eq!(
            world.block_state(DIM, growth),
            Some(if wants_shrieker {
                helpers::shrieker_state(false, false)
            } else {
                helpers::sensor_state("inactive", 0, false)
            }),
            "the growth block landed above the cursor"
        );

        // The spreader's `playSound` lands in the level's sink; the next tick
        // is where a block-entity body's events leave. The cursor list is
        // emptied first so that tick's own catalyst pass cannot spread again,
        // which leaves the queued growth sound as the only one it can carry.
        charge_spreader(&mut world, &[]);
        let flushed = world.tick(receiver.drain());
        let _ = sender;
        let sounds = sounds_sent(&flushed);
        let growth_sound = vanilla_sound_event_registry_id(expected)
            .expect("the growth state's place sound is in the catalog");
        let spread_sound = vanilla_sound_event_registry_id(SCULK_BLOCK_SPREAD_SOUND)
            .expect("the spread sound is in the catalog");
        let named: Vec<(SoundEventHolder, Vec3d, f32, f32)> = sounds
            .iter()
            .map(|sound| {
                (
                    sound.sound.clone(),
                    sound.position,
                    sound.volume,
                    sound.pitch,
                )
            })
            .collect();
        assert_eq!(
            named,
            vec![
                // `SculkSpreader`, at the same cursor cell.
                (
                    SoundEventHolder::Reference {
                        registry_id: spread_sound
                    },
                    Vec3d {
                        x: 10.5,
                        y: 64.5,
                        z: 0.5,
                    },
                    1.0,
                    1.0,
                ),
                // `SculkBlock`, after the write at `:45`.
                (
                    SoundEventHolder::Reference {
                        registry_id: growth_sound
                    },
                    Vec3d {
                        x: 10.5,
                        y: 64.5,
                        z: 0.5,
                    },
                    1.0,
                    1.0,
                ),
            ],
            "the spread sound the cursor's own update plays first, then the \
             drawn growth state's place sound, both at the cursor's cell"
        );
        assert!(sounds
            .iter()
            .all(|sound| sound.source == SoundSource::Blocks));
    }
}
