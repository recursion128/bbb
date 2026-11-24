use super::*;

#[test]
fn production_mineshaft_structure_placement_preserves_region_metadata() {
    let program = overworld_program(true, 0);
    let plan = program.structures.as_ref().unwrap();
    let source_chunk = [-128, -76];
    let graph = plan
        .mineshaft_piece_graph(
            "minecraft:mineshaft",
            0,
            source_chunk[0],
            source_chunk[1],
            -64,
            63,
            &StructureFlatHeight,
        )
        .unwrap();
    let targets: Vec<[i32; 2]> = graph
        .pieces
        .iter()
        .filter(|piece| {
            matches!(
                piece.kind,
                bbb_server_worldgen::noise::mineshaft::MineshaftPieceKind::Corridor {
                    spider_corridor: true,
                    ..
                }
            )
        })
        .flat_map(|piece| {
            let bb = piece.bounding_box;
            (bb.min.x.div_euclid(16)..=bb.max.x.div_euclid(16)).flat_map(move |x| {
                (bb.min.z.div_euclid(16)..=bb.max.z.div_euclid(16)).map(move |z| [x, z])
            })
        })
        .collect();
    assert!(
        !targets.is_empty(),
        "the supplied mine has a cave-spider corridor"
    );
    let start = MineshaftStructureStart::from_graph(plan, graph, 0).unwrap();
    let place = |target: [i32; 2]| {
        let invocation = plan
            .decoration_invocations_for_chunk(0, target[0], target[1])
            .into_iter()
            .find(|invocation| invocation.structure == "minecraft:mineshaft")
            .unwrap();
        let source = bbb_server_worldgen::noise::structure_starts::NoiseStructureChunkData::from_supplied_starts(
            source_chunk[0], source_chunk[1], [], [],
        ).with_mineshaft_starts([start.clone()]);
        let mut cache =
            crate::chunk::noise_generator::features::tests::structure_cache(target, source);
        crate::chunk::noise_generator::features::tests::with_feature_region(
            &program,
            target,
            |region, _source| {
                use bbb_server_worldgen::WorldGenLevel;
                let stone = default_state("minecraft:stone");
                for z in (target[1] - 1) * 16..(target[1] + 2) * 16 {
                    for x in (target[0] - 1) * 16..(target[0] + 2) * 16 {
                        for y in region.get_min_y()..=region.get_max_y() {
                            let pos = bbb_protocol::packets::BlockPos { x, y, z };
                            if bbb_protocol::block_state_types::vanilla_block_state_identity(
                                region.get_block(pos),
                            )
                            .is_some_and(|state| {
                                matches!(state.name.as_str(), "minecraft:water" | "minecraft:lava")
                            }) {
                                region.set_block(pos, stone, 2);
                            }
                        }
                    }
                }
                program
                    .structure_generation_context()?
                    .place_referenced_structure::<MineshaftStructureStart>(
                        &mut cache,
                        &invocation,
                        region,
                    )?;
                Ok(())
            },
        )
    };
    let (store, chunk) = targets
        .into_iter()
        .find_map(|target| {
            let store = place(target);
            let chunk = store.loaded_chunks().find(|&chunk| {
                store.block_entities().chunk(chunk).next().is_some()
                    || !store.proto_entity_records(chunk.0, chunk.1).is_empty()
            })?;
            Some((store, chunk))
        })
        .expect("the mine produces saved metadata in its placement region");
    let records = store
        .block_entities()
        .chunk(chunk)
        .map(crate::block_entity::saved_record)
        .collect::<bbb_server_types::Result<Vec<_>>>()
        .unwrap();
    let entities = store.proto_entity_records(chunk.0, chunk.1);
    let parsed = crate::chunk::noise_generator::features::tests::parsed_carvers(
        &store,
        [chunk.0, chunk.1],
        &program,
    );
    assert_eq!(parsed.block_entities(), records);
    assert_eq!(parsed.entities().unwrap_or_default(), entities);
}
