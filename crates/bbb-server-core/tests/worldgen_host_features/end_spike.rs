use super::seam::SeamLevel;
use super::support::{default_state, place_feature, retained_from_json};
use super::{catalog_default_state, FEATURE_SET_BLOCK};
use bbb_protocol::packets::BlockPos;

fn explicit_spike_json(guarded: bool) -> String {
    format!(
        r#"{{"type":"minecraft:end_spike","config":{{"crystal_invulnerable":true,"spikes":[{{"centerX":4,"centerZ":-8,"radius":1,"height":68,"guarded":{guarded}}}],"crystal_beam_target":[0,70,0]}}}}"#
    )
}

#[test]
fn explicit_spike_writes_column_cage_crystal_bedrock_and_fire() {
    let mut level = SeamLevel::admitting();
    let retained = retained_from_json(explicit_spike_json(true).as_bytes());
    let origin = BlockPos { x: 4, y: 64, z: -8 };
    assert!(place_feature(&retained, &mut level, origin));

    let sets = level.inner.sets.borrow().clone();
    let obsidian = default_state("minecraft:obsidian");
    let air = default_state("minecraft:air");
    let min_y = level.inner.min_y;
    // Column: every cell within the radius-1 disc below height 68 is
    // obsidian, everything else above y=65 is air.
    for (pos, state, flags) in sets
        .iter()
        .take_while(|(_, state, _)| *state == obsidian || *state == air)
    {
        assert_eq!(*flags, FEATURE_SET_BLOCK);
        let inside = (pos.x - 4).pow(2) + (pos.z + 8).pow(2) <= 2;
        if *state == obsidian {
            assert!(inside && pos.y < 68 && pos.y >= min_y, "{pos:?}");
        } else {
            assert!(!(inside && pos.y < 68) && pos.y > 65, "{pos:?}");
        }
    }
    let iron_bars = catalog_default_state("minecraft:iron_bars");
    let cage: Vec<_> = sets
        .iter()
        .filter(|(_, state, _)| {
            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(*state)
                == bbb_protocol::block_state_types::vanilla_block_state_block_type_id(iron_bars)
        })
        .collect();
    // 5x5 walls over four rows minus the 3x3 interior of the lower three.
    assert_eq!(cage.len(), 4 * 25 - 3 * 9);
    let crystals = level.end_crystals.borrow().clone();
    assert_eq!(crystals.len(), 1);
    assert_eq!(crystals[0].0, [4.5, 69.0, -7.5]);
    assert_eq!(crystals[0].2, Some(BlockPos { x: 0, y: 70, z: 0 }));
    assert!(crystals[0].3);
    let tail = &sets[sets.len() - 2..];
    assert_eq!(
        tail[0],
        (
            BlockPos { x: 4, y: 68, z: -8 },
            default_state("minecraft:bedrock"),
            FEATURE_SET_BLOCK
        )
    );
    assert_eq!(
        tail[1],
        (
            BlockPos { x: 4, y: 69, z: -8 },
            catalog_default_state("minecraft:fire"),
            FEATURE_SET_BLOCK
        )
    );
}
