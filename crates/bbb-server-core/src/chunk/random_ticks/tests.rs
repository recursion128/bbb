use crate::random::GameRandom;
use crate::random_ticks::{state_fluid_randomly_ticks, state_randomly_ticks};
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_id_by_name_and_properties,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_LAVA_ID, VANILLA_BLOCK_STONE_ID,
};
use std::collections::BTreeMap;

use super::{RandomTickSampler, SECTION_SIDE};
use bbb_server_types::chunk_data::ChunkColumn;

const PLAINS_BIOME_ID: i32 = 0;

fn wheat_state(age: u8) -> i32 {
    let mut properties = BTreeMap::new();
    properties.insert("age".to_string(), age.to_string());
    vanilla_block_state_id_by_name_and_properties("minecraft:wheat", &properties)
        .expect("built-in 26.1 wheat state")
}
fn lava_state() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_LAVA_ID).expect("built-in 26.1 lava")
}
fn stone_state() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_STONE_ID).expect("built-in 26.1 stone")
}
fn air_state() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_AIR_ID).expect("built-in 26.1 air")
}
fn column(min_y: i32, section_count: usize) -> ChunkColumn {
    ChunkColumn::new(min_y, section_count, air_state(), PLAINS_BIOME_ID)
}
fn fill_section(column: &mut ChunkColumn, section_index: usize, state: i32) {
    let base = column.min_y() + section_index as i32 * SECTION_SIDE;
    for y in base..base + SECTION_SIDE {
        for x in 0..SECTION_SIDE {
            for z in 0..SECTION_SIDE {
                column.set_block_state(x, y, z, state);
            }
        }
    }
}
#[test]
fn sampled_positions_are_offset_from_the_chunks_minimum_block_corner() {
    let mut column = column(0, 2);
    fill_section(&mut column, 0, wheat_state(0));
    let positions = GameRandom::with_seed(1);
    let mut sampler = RandomTickSampler::new(2, -3, 3);
    let mut count = 0;
    while let Some(pos) = sampler.next_position(&column, &positions) {
        assert!((32..48).contains(&pos.x));
        assert!((0..16).contains(&pos.y));
        assert!((-48..-32).contains(&pos.z));
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn every_randomly_ticking_section_is_sampled_random_tick_speed_times() {
    let mut column = column(0, 2);
    fill_section(&mut column, 0, wheat_state(0));
    fill_section(&mut column, 1, wheat_state(0));
    let positions = GameRandom::with_seed(1);
    let mut sampler = RandomTickSampler::new(0, 0, 2);
    let mut counts = [0; 2];
    while let Some(pos) = sampler.next_position(&column, &positions) {
        counts[(pos.y / SECTION_SIDE) as usize] += 1;
    }
    assert_eq!(counts, [2, 2]);
}

#[test]
fn a_random_tick_speed_of_zero_draws_no_positions_at_all() {
    let mut column = column(0, 2);
    fill_section(&mut column, 0, wheat_state(0));
    let positions = GameRandom::with_seed(1);
    let mut sampler = RandomTickSampler::new(0, 0, 0);
    assert!(sampler.next_position(&column, &positions).is_none());
}

#[test]
fn only_sections_whose_ticking_counters_are_nonzero_are_sampled() {
    let mut column = column(0, 3);
    fill_section(&mut column, 0, stone_state());
    fill_section(&mut column, 1, wheat_state(0));
    let positions = GameRandom::with_seed(1);
    let mut sampler = RandomTickSampler::new(0, 0, 3);
    let mut count = 0;
    while let Some(pos) = sampler.next_position(&column, &positions) {
        assert!((16..32).contains(&pos.y));
        count += 1;
    }
    assert_eq!(count, 3);
}

#[test]
fn a_section_holding_only_fully_grown_wheat_is_skipped_entirely() {
    let mut column = column(0, 1);
    fill_section(&mut column, 0, wheat_state(7));
    let positions = GameRandom::with_seed(1);
    let mut sampler = RandomTickSampler::new(0, 0, 3);
    assert!(sampler.next_position(&column, &positions).is_none());
}

#[test]
fn growing_wheat_randomly_ticks_but_fully_grown_wheat_does_not() {
    assert!(state_randomly_ticks(wheat_state(0)));
    assert!(!state_randomly_ticks(wheat_state(7)));
}

#[test]
fn lava_randomly_ticks_as_both_block_and_fluid() {
    let lava = lava_state();
    assert!(state_randomly_ticks(lava));
    assert!(state_fluid_randomly_ticks(lava));
    let mut column = column(0, 1);
    fill_section(&mut column, 0, lava);
    assert!(column.chunk_section(0).is_randomly_ticking_blocks());
    assert!(column.chunk_section(0).is_randomly_ticking_fluids());
}

#[test]
fn wheat_never_randomly_ticks_as_fluid() {
    assert!(!state_fluid_randomly_ticks(wheat_state(0)));
}
