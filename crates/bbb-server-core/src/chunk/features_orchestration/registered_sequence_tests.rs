use bbb_protocol::NbtSummaryValue;
use bbb_server_worldgen::noise::mineshaft::MineshaftStructureStart;
use bbb_server_worldgen::noise::structure_pieces::JigsawHeightAccess;
use bbb_server_worldgen::noise::structure_starts::{JigsawStructureStart, NoiseStructureChunkData};
use bbb_server_worldgen::noise::structures::{
    NoiseStructurePlan, StructureDecorationStep, StructureHeightmap,
};

use super::registered_sequence::{registered_invocations_for_test, registered_owner_for_test};
use crate::chunk::noise_generator::test_support::noise_stem;
use crate::chunk::noise_generator::{
    compile_noise_final_density, overworld_program, overworld_startup,
};
use crate::chunk::structure_status::NoiseStructureStatusCache;

const TARGET: [i32; 2] = [0, 0];

struct FlatHeight;

impl JigsawHeightAccess for FlatHeight {
    fn first_free_height(&self, _x: i32, _z: i32, _heightmap: StructureHeightmap) -> Option<i32> {
        Some(64)
    }
}

fn plan() -> NoiseStructurePlan {
    let startup = overworld_startup(true);
    let possible_biomes = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .collect::<Vec<_>>();
    NoiseStructurePlan::from_startup(&startup, &possible_biomes).unwrap()
}

fn jigsaw_start(plan: &NoiseStructurePlan, structure_id: &str) -> JigsawStructureStart {
    let startup = overworld_startup(true);
    let graph = plan
        .jigsaw_piece_graph(
            &startup,
            structure_id,
            0,
            TARGET[0],
            TARGET[1],
            -64,
            384,
            &FlatHeight,
        )
        .unwrap();
    JigsawStructureStart::from_graph(plan, graph, 0).unwrap()
}

fn multi_family_cache() -> NoiseStructureStatusCache {
    let plan = plan();
    let village = jigsaw_start(&plan, "minecraft:village_plains");
    let outpost = jigsaw_start(&plan, "minecraft:pillager_outpost");
    let mineshaft = MineshaftStructureStart::from_graph(
        &plan,
        plan.mineshaft_piece_graph(
            "minecraft:mineshaft",
            0,
            TARGET[0],
            TARGET[1],
            -64,
            63,
            &FlatHeight,
        )
        .unwrap(),
        0,
    )
    .unwrap();
    let starts =
        NoiseStructureChunkData::from_supplied_starts(TARGET[0], TARGET[1], [village, outpost], [])
            .with_mineshaft_starts([mineshaft]);
    let (references, sources) = NoiseStructureChunkData::references_from_complete_source_window(
        &starts,
        |source_x, source_z| {
            Ok::<_, &str>(if [source_x, source_z] == TARGET {
                starts.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
            })
        },
    )
    .unwrap();
    let mut cache = NoiseStructureStatusCache::new(true);
    for source in sources.into_values() {
        cache.restore_chunk_component(source).unwrap();
    }
    cache.restore_chunk_component(references).unwrap();
    cache
}

#[test]
fn multi_registered_structure_features_follow_step_then_registry_index() {
    let program = overworld_program(true, 0);
    let invocations =
        registered_invocations_for_test(&multi_family_cache(), &program, TARGET).unwrap();
    assert_eq!(invocations.len(), 3);
    assert_eq!(
        invocations[0].step,
        StructureDecorationStep::UndergroundStructures.ordinal()
    );
    assert_eq!(invocations[0].structure, "minecraft:mineshaft");
    assert_eq!(
        invocations[1].step,
        StructureDecorationStep::SurfaceStructures.ordinal()
    );
    assert_eq!(
        invocations[2].step,
        StructureDecorationStep::SurfaceStructures.ordinal()
    );
    assert!(invocations[1].step_index < invocations[2].step_index);
    assert_ne!(invocations[1].feature_seed, invocations[2].feature_seed);
}

#[test]
fn dimension_neutral_registered_structure_features_use_the_frozen_profile() {
    let mut stem = noise_stem(NbtSummaryValue::string("minecraft:nether"));
    stem.dimension_type = "minecraft:overworld".to_string();
    let program = std::sync::Arc::new(
        compile_noise_final_density(
            &stem,
            overworld_startup(true),
            0,
            "dimensions.minecraft:overworld.generator",
        )
        .unwrap(),
    );
    assert!(registered_owner_for_test(&program, "minecraft:fortress"));

    let (target, fortress_starts) = (0..=63)
        .flat_map(|chunk_z| (0..=63).map(move |chunk_x| [chunk_x, chunk_z]))
        .find_map(|target| {
            let (_, fortress) = program
                .generate_nether_complex_structure_starts_component(target[0], target[1], &[], &[])
                .unwrap();
            (!fortress.is_empty()).then_some((target, fortress))
        })
        .expect("seed zero must select a fortress in the first four regions");
    let source = NoiseStructureChunkData::from_supplied_starts(target[0], target[1], [], [])
        .with_fortress_starts(fortress_starts.clone());
    let (references, sources) = NoiseStructureChunkData::references_from_complete_source_window(
        &source,
        |source_x, source_z| {
            Ok::<_, &'static str>(if [source_x, source_z] == target {
                source.clone()
            } else {
                NoiseStructureChunkData::from_supplied_starts(source_x, source_z, [], [])
            })
        },
    )
    .unwrap();
    let mut cache = NoiseStructureStatusCache::new(true);
    for source in sources.into_values() {
        cache.restore_chunk_component(source).unwrap();
    }
    cache.restore_chunk_component(references).unwrap();
    let invocations = registered_invocations_for_test(&cache, &program, target).unwrap();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].structure, "minecraft:fortress");

    let _store = crate::chunk::noise_generator::features::tests::with_feature_region(
        &program,
        target,
        |region, _source| {
            program.structure_generation_context().unwrap()
        .place_referenced_structure::<bbb_server_worldgen::noise::nether_fortress::FortressStructureStart>(
            &mut cache, &invocations[0], region,
        )?;
            Ok(())
        },
    );
}
