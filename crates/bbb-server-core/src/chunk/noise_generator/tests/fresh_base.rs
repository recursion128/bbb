use super::*;

#[test]
fn saved_end_stem_compiles_density_and_biomes_through_fresh_base() {
    let startup = vanilla_startup();
    let program = compile_noise_final_density(
        &end_noise_stem(),
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:the_end.generator",
    )
    .unwrap();
    assert_eq!(program.dimension_type_id(), "minecraft:the_end");
    assert_eq!(program.settings_id(), "minecraft:end");
    assert_eq!((program.min_y(), program.height()), (0, 128));
    assert_eq!((program.cell_width(), program.cell_height()), (8, 4));
    for (x, y, z) in [(0, 0, 0), (16, 32, -16), (-73, 64, 91)] {
        assert!(program.compute_single_point(x, y, z).is_finite());
        assert!(program.compute_interpolated_point(x, y, z).is_finite());
    }

    let generated = program.generate_fresh_base_chunk(0, 0).unwrap();
    let end_biome = startup
        .registries
        .registry("minecraft:worldgen/biome")
        .unwrap()
        .elements
        .iter()
        .position(|element| element.id == "minecraft:the_end")
        .unwrap() as i32;
    for quart_y in 0..64 {
        assert_eq!(
            generated.column().get_noise_biome(0, quart_y, 0),
            Some(end_biome)
        );
    }
}
