use super::*;

use bbb_server_worldgen::noise::beardifier::{JigsawBeardifier, TerrainAdjustingStart};
use bbb_server_worldgen::noise::structure_pieces::JigsawBoundingBox;
use bbb_server_worldgen::noise::structures::StructureTerrainAdjustment;

fn beardifier_marker() -> serde_json::Value {
    serde_json::json!({"type": "minecraft:beardifier"})
}

fn constant(value: f64) -> serde_json::Value {
    serde_json::json!({
        "type": "minecraft:constant",
        "argument": value,
    })
}

fn program(
    id: &str,
    final_density: serde_json::Value,
) -> Result<super::super::NoiseFinalDensityProgram, String> {
    let mut startup = Arc::new(fresh_overworld_startup(true));
    let registry = &mut Arc::get_mut(&mut startup)
        .expect("the test has not shared its startup generation")
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:worldgen/noise_settings")
        .unwrap()
        .elements;
    let mut custom = registry
        .iter()
        .find(|element| element.id == "minecraft:overworld")
        .unwrap()
        .clone();
    let mut settings: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    settings["aquifers_enabled"] = serde_json::json!(false);
    settings["ore_veins_enabled"] = serde_json::json!(false);
    settings["noise_router"]["final_density"] = final_density;
    custom.id = id.to_string();
    custom.source_pack = "file/beardifier_density".to_string();
    custom.source_json = serde_json::to_vec(&settings).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(settings)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    publish_worldgen_element(
        &mut Arc::get_mut(&mut startup).unwrap().registries,
        "minecraft:worldgen/noise_settings",
        custom,
    );
    compile_noise_final_density(
        &with_settings(direct_noise_stem(direct_biomes()), id),
        startup,
        42,
        "generator",
    )
    .map_err(|error| error.to_string())
}

fn terrain_beardifier() -> (JigsawBeardifier, (i32, i32, i32), f64) {
    let starts = [TerrainAdjustingStart::Rigid {
        terrain_adjustment: StructureTerrainAdjustment::Bury,
        pieces: vec![JigsawBoundingBox {
            min: BlockPos { x: 0, y: 68, z: 0 },
            max: BlockPos { x: 6, y: 76, z: 6 },
        }],
    }];
    let beardifier = JigsawBeardifier::from_ordered_terrain_starts(0, 0, &starts);
    let (position, contribution) = (64..96)
        .flat_map(|y| (0..=6).flat_map(move |x| (0..=6).map(move |z| (x, y, z))))
        .find_map(|position @ (x, y, z)| {
            let contribution = beardifier.sample(x, y, z);
            (contribution != 0.0).then_some((position, contribution))
        })
        .expect("the BURY box contributes above sea level");
    (beardifier, position, contribution)
}

#[test]
fn source_exact_beardifier_density_is_zero_raw_and_bound_twice_in_final_density() {
    let (beardifier, (x, y, z), contribution) = terrain_beardifier();
    let base = -1.5 * contribution;
    let baseline = program("example:synthetic_beardifier", constant(base)).unwrap();
    let explicit = program(
        "example:explicit_beardifier",
        serde_json::json!({
            "type": "minecraft:add",
            "argument1": constant(base),
            "argument2": beardifier_marker(),
        }),
    )
    .unwrap();

    assert_eq!(
        explicit.compute_single_point(x, y, z).to_bits(),
        base.to_bits(),
        "the unit marker stays zero outside a bound NoiseChunk"
    );
    let baseline = baseline.generate_material_chunk_with_beardifier(0, 0, -64, 384, &beardifier);
    let explicit = explicit.generate_material_chunk_with_beardifier(0, 0, -64, 384, &beardifier);
    assert_ne!(
        baseline.state(x, y, z),
        explicit.state(x, y, z),
        "explicit plus synthetic Beardifier must cross the chosen density boundary"
    );
}
