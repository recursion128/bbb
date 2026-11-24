use super::catalog_default_state as default_state;
use super::support::RecordingLevel;
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::packets::BlockPos;
use bbb_server_worldgen::random::WorldgenRandom;
use bbb_server_worldgen::structure_processor::{
    process_block_infos, StructureBlockInfo, StructureProcessor,
};
fn state(name: &str, properties: &[(&str, &str)]) -> i32 {
    let properties = properties
        .iter()
        .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
        .collect();
    vanilla_block_state_id_by_name_and_properties(name, &properties).expect("state exists")
}
fn info(pos: BlockPos, state: i32) -> StructureBlockInfo {
    StructureBlockInfo {
        pos,
        state,
        nbt: None,
    }
}
fn run(
    level: &RecordingLevel,
    processors: &[StructureProcessor],
    blocks: &[StructureBlockInfo],
    seed: i64,
) -> Option<Vec<StructureBlockInfo>> {
    process_block_infos(
        level,
        BlockPos { x: 0, y: 0, z: 0 },
        BlockPos { x: 0, y: 0, z: 0 },
        processors,
        blocks,
        &mut WorldgenRandom::with_seed(seed),
        |pos| pos,
    )
}
#[test]
fn lava_submerged_processor_replaces_partial_collision_shape() {
    let level = RecordingLevel::filled(default_state("minecraft:lava"));
    let fence = state(
        "minecraft:oak_fence",
        &[
            ("east", "false"),
            ("north", "false"),
            ("south", "false"),
            ("waterlogged", "false"),
            ("west", "false"),
        ],
    );
    assert_eq!(
        run(
            &level,
            &[StructureProcessor::LavaSubmerged],
            &[info(BlockPos { x: 0, y: 0, z: 0 }, fence)],
            1
        )
        .unwrap()[0]
            .state,
        default_state("minecraft:lava")
    );
}
