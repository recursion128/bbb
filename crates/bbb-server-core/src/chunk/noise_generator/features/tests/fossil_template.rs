use bbb_registry_codec::legacy_random::LegacyRandomSource;

use super::*;

#[test]
fn fossil_structure_template_owner() {
    let program = overworld_program(true, 0);
    let resources = program
        .feature_resources
        .as_ref()
        .expect("Overworld FEATURES resources");
    let retained =
        resources.registries.worldgen.configured_features["minecraft:fossil_coal"].clone();

    with_feature_region(&program, [0, 0], |region, _source| {
        assert!(bbb_server_worldgen::place_configured_feature(
            &retained,
            region,
            BlockPos { x: 8, y: 0, z: 8 },
            &mut LegacyRandomSource::with_seed(256),
        ));
        Ok(())
    });
}
