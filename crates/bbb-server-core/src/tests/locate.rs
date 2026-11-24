//! Live `/locate biome` world settlement.

use super::*;
use crate::commands::{LocateKind, LocateRequest, LocateSearchResult};

fn request(ids: &[&str], y: i32) -> LocateRequest {
    LocateRequest {
        kind: LocateKind::Biome,
        level: Java26BuiltinDimension::Overworld.into(),
        origin: BlockPos { x: 0, y, z: 0 },
        target_display: ids.first().copied().unwrap_or("#empty").to_string(),
        allowed_ids: ids.iter().map(|id| (*id).to_string()).collect(),
        target_is_tag: ids.len() > 1,
    }
}

#[test]
fn empty_target_is_a_real_not_found_result() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    assert_eq!(
        world.locate_live_target(&request(&[], 70)),
        LocateSearchResult::NotFound
    );
}

#[test]
fn unknown_biome_fails_closed_before_search() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    assert_eq!(
        world.locate_live_target(&request(&["minecraft:not_a_biome"], 70)),
        LocateSearchResult::Unavailable
    );
}

#[test]
fn known_biome_uses_the_live_world_result() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    for y in [-10_000, 10_000] {
        let result = world.locate_live_target(&request(&["minecraft:plains"], y));
        assert_eq!(
            result,
            LocateSearchResult::Found {
                pos: BlockPos {
                    x: 0,
                    y: if y < 0 { -63 } else { 320 },
                    z: 0,
                },
                resource_id: "minecraft:plains".to_string(),
            }
        );
    }
}

#[test]
fn tag_projection_returns_the_actual_live_biome_id() {
    let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
    let result = world.locate_live_target(&request(&["minecraft:plains", "minecraft:forest"], 70));
    assert_eq!(
        result,
        LocateSearchResult::Found {
            pos: BlockPos { x: 0, y: 70, z: 0 },
            resource_id: "minecraft:plains".to_string(),
        }
    );
}
