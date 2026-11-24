use super::*;

fn shadow_test_store(entity_type_id: i32) -> WorldStore {
    let mut store = WorldStore::with_dimension(crate::WorldDimension {
        min_y: 0,
        height: 16,
    });
    store.insert_decoded_chunk(empty_test_chunk());
    assert!(store.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos { x: 1, y: 3, z: 1 },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    store.apply_add_entity(ProtocolAddEntity {
        position: ProtocolVec3d {
            x: 1.5,
            y: 4.0,
            z: 1.5,
        },
        ..protocol_add_entity_with_type(51, entity_type_id)
    });
    store
}

fn shadow_sources(store: &WorldStore, enabled: bool) -> Vec<EntityShadowRenderSourceState> {
    shadow_sources_with_context(
        store,
        enabled,
        EntityVec3 {
            x: 1.5,
            y: 5.62,
            z: 1.5,
        },
        0,
    )
}

fn shadow_sources_with_context(
    store: &WorldStore,
    enabled: bool,
    camera_position: EntityVec3,
    sky_darken: u8,
) -> Vec<EntityShadowRenderSourceState> {
    let candidates = store.entity_render_candidates_at_partial_tick(0.0);
    let model_sources =
        store.entity_model_sources_for_candidates_at_partial_tick(&candidates, &candidates, 0.0);
    let mut shadows = Vec::new();
    store.fill_entity_shadow_render_sources_for_model_sources(
        &candidates,
        &model_sources,
        0.0,
        camera_position,
        sky_darken,
        enabled,
        &mut shadows,
    );
    shadows
}

#[test]
fn shadow_projection_rejects_low_light_and_distant_entities() {
    let store = shadow_test_store(VANILLA_ENTITY_TYPE_CHICKEN_ID);
    assert!(shadow_sources_with_context(
        &store,
        true,
        EntityVec3 {
            x: 1.5,
            y: 5.62,
            z: 1.5,
        },
        15,
    )
    .is_empty());
    assert!(shadow_sources_with_context(
        &store,
        true,
        EntityVec3 {
            x: 20.0,
            y: 5.62,
            z: 1.5,
        },
        0,
    )
    .is_empty());
}

#[test]
fn shadow_projection_samples_full_block_surface_and_vanilla_light() {
    let store = shadow_test_store(VANILLA_ENTITY_TYPE_CHICKEN_ID);
    let shadows = shadow_sources(&store, true);

    assert_eq!(shadows.len(), 1);
    assert_eq!(shadows[0].entity_id, 51);
    assert_eq!(
        shadows[0].origin,
        EntityVec3 {
            x: 1.5,
            y: 4.0,
            z: 1.5
        }
    );
    assert!(shadows[0].radius > 0.0);
    let piece = shadows[0]
        .pieces
        .iter()
        .find(|piece| piece.relative_position == [-0.5, 0.0, -0.5])
        .expect("stone top produces the centered shadow piece");
    assert_eq!(piece.shape_min, [0.0; 3]);
    assert_eq!(piece.shape_max, [1.0; 3]);
    assert!(piece.alpha > 0.0 && piece.alpha <= 1.0);
}

#[test]
fn shadow_projection_honors_startup_option_and_raw_invisibility() {
    let mut store = shadow_test_store(VANILLA_ENTITY_TYPE_CHICKEN_ID);
    assert!(shadow_sources(&store, false).is_empty());

    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![entity_invisible_data()],
    }));
    assert!(shadow_sources(&store, true).is_empty());
}

#[test]
fn slime_shadow_radius_uses_synced_size_override() {
    let mut store = shadow_test_store(VANILLA_ENTITY_TYPE_SLIME_ID);
    let (data_id, _) =
        vanilla_entity_metadata_slime_size_accessor_for_entity_type(VANILLA_ENTITY_TYPE_SLIME_ID)
            .unwrap();
    assert!(store.apply_set_entity_data(ProtocolSetEntityData {
        id: 51,
        values: vec![ProtocolEntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(4),
        }],
    }));

    assert_eq!(shadow_sources(&store, true)[0].radius, 1.0);
}
