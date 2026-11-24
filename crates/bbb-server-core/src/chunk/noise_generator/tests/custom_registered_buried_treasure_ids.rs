use super::*;
use crate::chunk::structure_status::NoiseStructureStatusCache;
use crate::chunk::NoiseFinalDensityProgram;
use bbb_server_worldgen::noise::buried_treasure::BuriedTreasureStructureStart;
use bbb_server_worldgen::noise::structures::{StructureDecorationStep, StructureStartSetOwner};

fn install_clone(
    startup: &mut DatapackStartup,
    registry_id: &str,
    source_id: &str,
    custom_id: &str,
    rewrite: impl FnOnce(&mut serde_json::Value),
) {
    let registry = startup
        .registries
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == registry_id)
        .unwrap();
    let mut custom = registry
        .elements
        .iter()
        .find(|element| element.id == source_id)
        .unwrap()
        .clone();
    let mut json: serde_json::Value = serde_json::from_slice(&custom.source_json).unwrap();
    rewrite(&mut json);
    custom.id = custom_id.to_string();
    custom.source_pack = "file/custom_registered_buried_treasure".to_string();
    custom.source_json = serde_json::to_vec(&json).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    registry.elements.push(custom);
}

fn startup(include_mixed_set: bool) -> Arc<DatapackStartup> {
    let mut startup = fresh_overworld_startup(true);
    for custom_id in [
        "test:z_buried_cache",
        "test:backup_buried_cache",
        "test:a_buried_cache",
    ] {
        install_clone(
            &mut startup,
            "minecraft:worldgen/structure",
            "minecraft:buried_treasure",
            custom_id,
            |_| {},
        );
    }
    install_clone(
        &mut startup,
        "minecraft:worldgen/structure_set",
        "minecraft:buried_treasures",
        "test:first_buried_caches",
        |json| {
            json["placement"]["spacing"] = serde_json::json!(1);
            json["placement"]["separation"] = serde_json::json!(0);
            json["placement"]["frequency"] = serde_json::json!(1.0);
            json["placement"]["salt"] = serde_json::json!(13579);
            json["structures"] = serde_json::json!([
                {"structure":"test:z_buried_cache","weight":2},
                {"structure":"test:backup_buried_cache","weight":3}
            ]);
        },
    );
    install_clone(
        &mut startup,
        "minecraft:worldgen/structure_set",
        "minecraft:buried_treasures",
        "test:second_buried_caches",
        |json| {
            json["placement"]["spacing"] = serde_json::json!(1);
            json["placement"]["separation"] = serde_json::json!(0);
            json["placement"]["frequency"] = serde_json::json!(1.0);
            json["placement"]["salt"] = serde_json::json!(24680);
            json["structures"] =
                serde_json::json!([{"structure":"test:a_buried_cache","weight":1}]);
        },
    );
    if include_mixed_set {
        install_clone(
            &mut startup,
            "minecraft:worldgen/structure_set",
            "minecraft:buried_treasures",
            "test:mixed_buried_caches",
            |json| {
                json["placement"]["spacing"] = serde_json::json!(1);
                json["placement"]["separation"] = serde_json::json!(0);
                json["placement"]["frequency"] = serde_json::json!(1.0);
                json["structures"] = serde_json::json!([
                    {"structure":"test:z_buried_cache","weight":1},
                    {"structure":"minecraft:igloo","weight":1}
                ]);
            },
        );
    }
    for (registry_id, removed_id) in [
        ("minecraft:worldgen/structure", "minecraft:buried_treasure"),
        (
            "minecraft:worldgen/structure_set",
            "minecraft:buried_treasures",
        ),
    ] {
        startup
            .registries
            .registries
            .iter_mut()
            .find(|registry| registry.registry_id == registry_id)
            .unwrap()
            .elements
            .retain(|element| element.id != removed_id);
    }
    Arc::new(startup)
}

fn program(include_mixed_set: bool) -> (Arc<DatapackStartup>, Arc<NoiseFinalDensityProgram>) {
    let startup = startup(include_mixed_set);
    let program = Arc::new(
        compile_noise_final_density(
            &overworld_noise_stem(),
            Arc::clone(&startup),
            0,
            "dimensions.minecraft:overworld.generator",
        )
        .unwrap(),
    );
    (startup, program)
}

fn references_for(
    target: [i32; 2],
    starts: Vec<BuriedTreasureStructureStart>,
) -> NoiseStructureChunkData {
    let target_starts = NoiseStructureChunkData::from_supplied_starts(target[0], target[1], [], [])
        .with_buried_treasure_starts(starts);
    NoiseStructureChunkData::references_from_complete_source_window(
        &target_starts,
        |source_x, source_z| {
            if [source_x, source_z] == target {
                Ok::<_, &'static str>(target_starts.clone())
            } else {
                Ok(NoiseStructureChunkData::from_supplied_starts(
                    source_x,
                    source_z,
                    [],
                    [],
                ))
            }
        },
    )
    .unwrap()
    .0
}

fn first_custom_buried_treasure_target(
    program: &NoiseFinalDensityProgram,
    radius: i32,
) -> ([i32; 2], Vec<BuriedTreasureStructureStart>) {
    let plan = program.structures.as_ref().unwrap();
    for chunk_z in -radius..=radius {
        for chunk_x in -radius..=radius {
            if plan
                .random_spread_attempt_order("test:first_buried_caches", 0, chunk_x, chunk_z)
                .is_none()
            {
                continue;
            }
            let starts = program
                .generate_buried_treasure_structure_starts_component(chunk_x, chunk_z, &[])
                .unwrap();
            if starts.len() == 2 {
                return ([chunk_x, chunk_z], starts);
            }
        }
    }
    panic!("no custom buried-treasure target in radius {radius}")
}

#[test]
fn custom_registered_buried_treasure_ids_preserve_selection_restore_and_features() {
    let (startup, program) = program(false);
    let plan = program.structures.as_ref().unwrap();
    for set_id in ["test:first_buried_caches", "test:second_buried_caches"] {
        assert_eq!(
            plan.structure_start_owner(plan.set(set_id).unwrap()),
            StructureStartSetOwner::BuriedTreasure
        );
    }
    assert!(!program.owns_buried_treasure_structure(BURIED_TREASURE_ID));

    let (target, generated) = first_custom_buried_treasure_target(&program, 128);
    let first_attempts = plan
        .random_spread_attempt_order("test:first_buried_caches", 0, target[0], target[1])
        .unwrap();
    assert_eq!(first_attempts.len(), 2);
    assert_eq!(generated.len(), 2);
    assert_eq!(generated[0].structure_id(), first_attempts[0]);
    assert_eq!(generated[1].structure_id(), "test:a_buried_cache");
    assert!(generated.iter().all(|start| {
        start.piece().position()
            == BlockPos {
                x: target[0] * 16 + 9,
                y: 90,
                z: target[1] * 16 + 9,
            }
    }));

    let source = NoiseStructureChunkData::from_supplied_starts(target[0], target[1], [], [])
        .with_buried_treasure_starts(generated.clone());
    let _ = &startup;
    let restored = program
        .restored_structure_chunk_data(&source.to_nbt(), target[0], target[1])
        .unwrap();
    assert_eq!(restored, source);
    assert_eq!(restored.buried_treasure_starts().len(), 2);

    let references = references_for(target, generated.clone());
    for start in &generated {
        assert_eq!(references.references()[start.structure_id()].len(), 1);
        let invocation = program
            .buried_treasure_structure_decoration_invocation_for(target, start.structure_id())
            .unwrap();
        assert_eq!(
            invocation.step,
            StructureDecorationStep::UndergroundStructures.ordinal()
        );
    }

    let selected = generated[0].structure_id().to_string();
    let mut cache = NoiseStructureStatusCache::new(true);
    cache.restore_chunk_component(references).unwrap();
    let context = program.structure_generation_context().unwrap();
    let invocation = context
        .structure_invocation(
            target,
            &selected,
            crate::chunk::noise_generator::features::StructurePlacementFamily::BuriedTreasure,
        )
        .unwrap();
    let _store = crate::chunk::noise_generator::features::tests::with_feature_region(
        &program,
        target,
        |region, _source| {
            context.place_referenced_structure::<BuriedTreasureStructureStart>(
                &mut cache,
                &invocation,
                region,
            )?;
            Ok(())
        },
    );
}
