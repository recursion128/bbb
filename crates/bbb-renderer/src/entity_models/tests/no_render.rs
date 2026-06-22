use super::*;

#[test]
fn no_render_entities_emit_no_geometry() {
    // Vanilla `NoopRenderer` entities (area effect cloud, marker, interaction) draw no model, so the
    // `NoRender` kind emits an empty mesh — exact parity, not a placeholder box.
    let mesh = entity_model_mesh(&[EntityModelInstance::no_render(10, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(mesh.opaque_faces, 0);
    assert!(mesh.vertices.is_empty());
    assert!(mesh.indices.is_empty());
}

#[test]
fn no_render_does_not_suppress_other_entities_in_the_batch() {
    // A `NoRender` instance in the batch contributes nothing but must not drop the others.
    let with_noop = entity_model_mesh(&[
        EntityModelInstance::no_render(11, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::leash_knot(12, [0.0, 64.0, 0.0], 0.0),
    ]);
    let knot_only =
        entity_model_mesh(&[EntityModelInstance::leash_knot(12, [0.0, 64.0, 0.0], 0.0)]);
    assert_eq!(with_noop.vertices.len(), knot_only.vertices.len());
    assert_eq!(with_noop.opaque_faces, knot_only.opaque_faces);
}
