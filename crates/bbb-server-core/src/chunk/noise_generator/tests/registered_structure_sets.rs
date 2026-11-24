use super::*;
use crate::chunk::NoiseFinalDensityProgram;
use bbb_server_worldgen::noise::structures::StructureStartSetOwner;

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
    custom.source_pack = "file/heterogeneous_registered_sets".to_string();
    custom.source_json = serde_json::to_vec(&json).unwrap().into();
    custom.source = Some(
        bbb_registry_codec::decode_direct_registry_json(&serde_json::to_vec(&(json)).unwrap())
            .unwrap(),
    );
    custom.codec_value = None;
    registry.elements.push(custom);
}

fn base_startup() -> DatapackStartup {
    let mut startup = fresh_overworld_startup(true);
    for (source, custom, biomes) in [
        ("minecraft:igloo", "test:empty_igloo", serde_json::json!([])),
        (
            "minecraft:mineshaft",
            "test:plains_mineshaft",
            serde_json::json!("minecraft:plains"),
        ),
        (
            "minecraft:buried_treasure",
            "test:plains_buried_treasure",
            serde_json::json!("minecraft:plains"),
        ),
    ] {
        install_clone(
            &mut startup,
            "minecraft:worldgen/structure",
            source,
            custom,
            |json| json["biomes"] = biomes,
        );
    }
    startup
}

fn install_set(startup: &mut DatapackStartup, id: &str, structures: serde_json::Value) {
    install_clone(
        startup,
        "minecraft:worldgen/structure_set",
        "minecraft:mineshafts",
        id,
        |json| {
            json["placement"]["frequency"] = serde_json::json!(1.0);
            json["placement"]["frequency_reduction_method"] = serde_json::json!("default");
            json["placement"]["salt"] = serde_json::json!(731_337);
            json["placement"]["spacing"] = serde_json::json!(1);
            json["placement"]["separation"] = serde_json::json!(0);
            json["structures"] = structures;
        },
    );
}

fn install_concentric_set(startup: &mut DatapackStartup, id: &str, structures: serde_json::Value) {
    install_clone(
        startup,
        "minecraft:worldgen/structure_set",
        "minecraft:strongholds",
        id,
        |json| json["structures"] = structures,
    );
}

fn program(startup: DatapackStartup) -> Arc<NoiseFinalDensityProgram> {
    let startup = Arc::new(startup);
    Arc::new(
        compile_noise_final_density(
            &direct_noise_stem(direct_biomes()),
            startup,
            0,
            "dimensions.minecraft:overworld.generator",
        )
        .unwrap(),
    )
}

#[test]
fn heterogeneous_registered_structure_sets_retry_in_source_order_and_stop_on_first_valid() {
    let mut startup = base_startup();
    install_set(
        &mut startup,
        "test:heterogeneous_retry",
        serde_json::json!([
            {"structure":"test:empty_igloo","weight":1},
            {"structure":"test:plains_mineshaft","weight":1},
            {"structure":"test:plains_buried_treasure","weight":1}
        ]),
    );
    let program = program(startup);
    let plan = program.structures.as_ref().unwrap();
    let set = plan.set("test:heterogeneous_retry").unwrap();
    assert_eq!(
        plan.structure_start_owner(set),
        StructureStartSetOwner::RegisteredRandomSpread
    );
    let target = (-32..=32)
        .flat_map(|x| (-32..=32).map(move |z| [x, z]))
        .find(|[x, z]| {
            plan.random_spread_attempt_order("test:heterogeneous_retry", 0, *x, *z)
                .is_some_and(|attempts| {
                    attempts
                        == [
                            "test:empty_igloo",
                            "test:plains_mineshaft",
                            "test:plains_buried_treasure",
                        ]
                })
        })
        .unwrap();
    let rings = program.structure_start_ring_positions().unwrap();
    let starts = program
        .generate_structure_starts_status_task(target[0], target[1], true, &rings)
        .unwrap();
    assert!(starts
        .mineshaft_starts()
        .iter()
        .any(|start| start.structure_id() == "test:plains_mineshaft"));
    assert!(!starts
        .igloo_starts()
        .iter()
        .any(|start| start.structure_id() == "test:empty_igloo"));
    assert!(!starts
        .buried_treasure_starts()
        .iter()
        .any(|start| start.structure_id() == "test:plains_buried_treasure"));
}

#[test]
fn heterogeneous_registered_structure_sets_existing_start_skips_whole_later_set() {
    let mut startup = base_startup();
    install_set(
        &mut startup,
        "test:first_mineshaft",
        serde_json::json!([
            {"structure":"test:plains_mineshaft","weight":1}
        ]),
    );
    install_set(
        &mut startup,
        "test:later_heterogeneous",
        serde_json::json!([
            {"structure":"test:plains_buried_treasure","weight":1},
            {"structure":"test:plains_mineshaft","weight":1}
        ]),
    );
    let program = program(startup);
    let rings = program.structure_start_ring_positions().unwrap();
    let starts = program
        .generate_structure_starts_status_task(0, 0, true, &rings)
        .unwrap();
    assert!(starts
        .mineshaft_starts()
        .iter()
        .any(|start| start.structure_id() == "test:plains_mineshaft"));
    assert!(!starts
        .buried_treasure_starts()
        .iter()
        .any(|start| start.structure_id() == "test:plains_buried_treasure"));
}

#[test]
fn heterogeneous_concentric_ring_set_retries_through_typed_start_owners() {
    let mut startup = base_startup();
    install_clone(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:shipwreck",
        "test:empty_ring_shipwreck",
        |json| json["biomes"] = serde_json::json!([]),
    );
    install_clone(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:stronghold",
        "test:plains_ring_stronghold",
        |json| json["biomes"] = serde_json::json!("minecraft:plains"),
    );
    install_concentric_set(
        &mut startup,
        "test:heterogeneous_ring_retry",
        serde_json::json!([
            {"structure":"test:empty_ring_shipwreck","weight":1},
            {"structure":"test:plains_ring_stronghold","weight":1}
        ]),
    );
    let program = program(startup);
    let plan = program.structures.as_ref().unwrap();
    let set = plan.set("test:heterogeneous_ring_retry").unwrap();
    assert_eq!(
        plan.structure_start_owner(set),
        StructureStartSetOwner::Unowned
    );
    let rings = program.structure_start_ring_positions().unwrap();
    let target = rings["test:heterogeneous_ring_retry"]
        .iter()
        .copied()
        .find(|[x, z]| {
            plan.weighted_attempt_order("test:heterogeneous_ring_retry", 0, *x, *z)
                == Some(vec![
                    "test:empty_ring_shipwreck".to_string(),
                    "test:plains_ring_stronghold".to_string(),
                ])
        })
        .unwrap();
    let starts = program
        .generate_structure_starts_status_task(target[0], target[1], true, &rings)
        .unwrap();
    assert!(!starts
        .shipwreck_starts()
        .iter()
        .any(|start| start.structure_id() == "test:empty_ring_shipwreck"));
    assert!(starts
        .stronghold_starts()
        .iter()
        .any(|start| start.structure_id() == "test:plains_ring_stronghold"));
}

#[test]
fn concentric_ring_set_skips_non_candidates_and_refuses_missing_positions() {
    let mut startup = base_startup();
    install_clone(
        &mut startup,
        "minecraft:worldgen/structure",
        "minecraft:shipwreck",
        "test:plains_ring_shipwreck",
        |json| json["biomes"] = serde_json::json!("minecraft:plains"),
    );
    install_concentric_set(
        &mut startup,
        "test:shipwreck_rings",
        serde_json::json!([
            {"structure":"test:plains_ring_shipwreck","weight":1}
        ]),
    );
    let program = program(startup);
    let rings = program.structure_start_ring_positions().unwrap();
    let positions = &rings["test:shipwreck_rings"];
    let target = (-4..=4)
        .flat_map(|x| (-4..=4).map(move |z| [x, z]))
        .find(|position| !positions.contains(position))
        .unwrap();
    let starts = program
        .generate_structure_starts_status_task(target[0], target[1], true, &rings)
        .unwrap();
    assert!(!starts
        .shipwreck_starts()
        .iter()
        .any(|start| start.structure_id() == "test:plains_ring_shipwreck"));

    let mut missing = rings;
    missing.remove("test:shipwreck_rings");
    let error = program
        .generate_structure_starts_status_task(target[0], target[1], true, &missing)
        .unwrap_err()
        .to_string();
    assert!(error.contains("test:shipwreck_rings"), "{error}");
    assert!(error.contains("no frozen positions"), "{error}");
}
