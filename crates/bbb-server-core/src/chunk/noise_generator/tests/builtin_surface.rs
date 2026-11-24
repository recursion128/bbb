use super::*;

#[test]
fn end_carvers_are_an_exact_empty_stage() {
    let startup = vanilla_startup();
    let stem = end_noise_stem();
    let program = compile_noise_final_density(
        &stem,
        Arc::clone(&startup),
        0,
        "dimensions.minecraft:the_end.generator",
    )
    .unwrap();
    for (chunk_x, chunk_z) in [(0, 0), (-1, 3), (90, -70)] {
        let surface = program
            .generate_fresh_surface_chunk(chunk_x, chunk_z)
            .unwrap();
        let carved = program
            .generate_fresh_carved_chunk(chunk_x, chunk_z)
            .unwrap();
        assert_eq!(carved.carving_mask(), Some([].as_slice()));
        let mut expected = surface;
        expected.carving_mask = Some(Vec::new());
        assert_eq!(
            carved, expected,
            "End CARVERS changed [{chunk_x},{chunk_z}]"
        );
    }
}
