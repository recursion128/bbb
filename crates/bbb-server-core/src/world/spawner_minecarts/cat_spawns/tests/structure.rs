use super::*;
use crate::chunk::noise_generator::overworld_program;
use crate::chunk::ChunkStore;
use bbb_server_persistence::{
    generation_state_chunk_document, Java26ChunkContext, Java26ChunkData,
};
use bbb_server_types::chunk_data::{ChunkColumn, ChunkGenerationState};
use bbb_server_worldgen::noise::scattered::{
    HorizontalDirection, ScatteredFeatureExtra, ScatteredFeatureKind, ScatteredFeaturePiece,
    ScatteredFeatureStructureStart,
};
use bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData;

const SOURCE: [i32; 2] = [1, 0];
const TARGET: [i32; 2] = [0, 0];
const BOX: [i32; 6] = [4, 64, 4, 10, 70, 12];

fn empty(chunk: [i32; 2]) -> NoiseStructureChunkData {
    NoiseStructureChunkData::from_supplied_starts(chunk[0], chunk[1], [], [])
}

fn structures() -> (NoiseStructureChunkData, NoiseStructureChunkData) {
    let piece = ScatteredFeaturePiece::new(
        ScatteredFeatureKind::SwampHut,
        7,
        7,
        9,
        64,
        HorizontalDirection::South,
        BOX,
        ScatteredFeatureExtra::SwampHut {
            witch: false,
            cat: false,
        },
    );
    let start = ScatteredFeatureStructureStart::from_parts("minecraft:swamp_hut", SOURCE, 0, piece);
    let source = empty(SOURCE).with_scattered_starts([start]);
    let target =
        NoiseStructureChunkData::references_from_complete_source_window(&empty(TARGET), |x, z| {
            Ok::<_, &str>(if [x, z] == SOURCE {
                source.clone()
            } else {
                empty([x, z])
            })
        })
        .unwrap()
        .0;
    assert!(target.references().contains_key("minecraft:swamp_hut"));
    (target, source)
}

fn install(store: &mut ChunkStore, chunk: [i32; 2], structures: &NoiseStructureChunkData) {
    let air = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_AIR_ID,
    )
    .unwrap();
    let column = ChunkColumn::new(-64, 24, air, 0);
    let state = ChunkGenerationState::Carvers {
        postprocessing_positions: vec![],
        carving_mask: vec![],
    };
    let document = generation_state_chunk_document(
        &column,
        chunk[0],
        chunk[1],
        &|_| Some("minecraft:plains".to_owned()),
        &state,
        Some(&structures.to_nbt()),
        &[],
    )
    .unwrap();
    let saved = Java26ChunkData::parse(
        document,
        Java26ChunkContext {
            expected_x: chunk[0],
            expected_z: chunk[1],
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap();
    store.insert_column(chunk[0], chunk[1], column);
    store.restore_saved_generation_metadata(&saved).unwrap();
}

fn hut_world(include_source: bool) -> ServerWorld {
    let mut world = world();
    let mut chunks = ChunkStore::new_overworld_carvers_with_structure_generation(
        overworld_program(true, 0),
        true,
    )
    .unwrap();
    chunks.install_registry_tags(world.registries.registry_tags_shared());
    let (target, source) = structures();
    install(&mut chunks, TARGET, &target);
    if include_source {
        install(&mut chunks, SOURCE, &source);
    }
    world
        .levels
        .get_mut(&world.require_level_id(&key()))
        .unwrap()
        .chunks = chunks;
    world
}

#[test]
fn cat_spawn_hut_resolves_remote_start_and_inclusive_piece_faces() {
    let mut world = hut_world(true);
    // StructureManager.java:109-146 checks the piece, not the source chunk.
    for pos in [
        BlockPos { x: 4, y: 64, z: 4 },
        BlockPos {
            x: 10,
            y: 70,
            z: 12,
        },
        BlockPos { x: 7, y: 67, z: 8 },
    ] {
        assert!(
            world.cat_spawn_in_hut(&world.require_level_id(&key()), pos),
            "{pos:?}"
        );
    }
    for pos in [
        BlockPos { x: 3, y: 67, z: 8 },
        BlockPos { x: 11, y: 67, z: 8 },
        BlockPos { x: 7, y: 63, z: 8 },
        BlockPos { x: 7, y: 71, z: 8 },
        BlockPos { x: 7, y: 67, z: 3 },
        BlockPos { x: 7, y: 67, z: 13 },
    ] {
        assert!(
            !world.cat_spawn_in_hut(&world.require_level_id(&key()), pos),
            "{pos:?}"
        );
    }
}

/// SwampHutPiece.java:115-125 via `Cat.finalizeSpawn` and the
/// `#cats_spawn_as_black` structure condition of `cat_variant/all_black`.
#[test]
fn a_generated_swamp_hut_cat_is_all_black() {
    use bbb_server_worldgen::noise::proto_chunk_entity::{
        mark_structure_finalize, ProtoChunkEntityRecord, StructureMobKind, StructureMobRecord,
    };
    let mut world = hut_world(true);
    let dimension = world.require_level_id(&key());
    let cat =
        StructureMobRecord::new(StructureMobKind::Cat, [7.5, 66.0, 8.5], [0, 0, 0, 9]).unwrap();
    let mut record = ProtoChunkEntityRecord::StructureMob(cat).entries();
    mark_structure_finalize(&mut record);
    world
        .level_mut_by_id(&dimension)
        .chunks
        .worldgen_entity_records_mut((TARGET[0], TARGET[1]))
        .unwrap()[0]
        .push(record);
    world.finalize_structure_mobs();
    let records = world
        .level_by_id(&dimension)
        .chunks
        .proto_entity_records(TARGET[0], TARGET[1])
        .to_vec();
    let variant = records
        .iter()
        .flatten()
        .find(|entry| entry.name == "variant")
        .map(|entry| entry.value.clone());
    assert_eq!(
        variant,
        Some(bbb_protocol::NbtSummaryValue::string("minecraft:all_black"))
    );
}
