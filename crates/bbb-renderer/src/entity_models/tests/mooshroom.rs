use super::*;

#[test]
fn mooshroom_mesh_matches_the_temperate_cow_body() {
    // Vanilla `MushroomCowRenderer` renders the mooshroom with the shared `CowModel` body
    // (`ModelLayers.MOOSHROOM` bakes to the same temperate `cowBodyLayer` as `ModelLayers.COW`), so the
    // mooshroom emits the exact temperate-cow mesh — the real cow body, not the generic quadruped.
    let mooshroom = entity_model_mesh(&[EntityModelInstance::mooshroom(
        700,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let cow = entity_model_mesh(&[EntityModelInstance::cow(700, [0.0, 64.0, 0.0], 0.0, false)]);
    assert_eq!(mooshroom.vertices, cow.vertices);
    assert_eq!(mooshroom.indices, cow.indices);

    // The temperate adult cow body is ten cubes → 60 faces / 240 vertices / 360 indices, COW_BROWN.
    assert_eq!(mooshroom.opaque_faces, 60);
    assert_eq!(mooshroom.vertices.len(), 240);
    assert_eq!(mooshroom.indices.len(), 360);
    assert!(mooshroom
        .vertices
        .iter()
        .any(|vertex| vertex.color == shade_color(COW_BROWN, 1.0)));
}

#[test]
fn baby_mooshroom_mesh_matches_the_baby_cow_body() {
    // The baby mooshroom uses `BabyCowModel.createBodyLayer()` (`ModelLayers.MOOSHROOM_BABY`), the same
    // body as the baby cow, so its mesh matches the baby cow's and is smaller than the adult.
    let baby = entity_model_mesh(&[EntityModelInstance::mooshroom(
        701,
        [0.0, 64.0, 0.0],
        0.0,
        true,
    )]);
    let baby_cow = entity_model_mesh(&[EntityModelInstance::cow(701, [0.0, 64.0, 0.0], 0.0, true)]);
    assert_eq!(baby.vertices, baby_cow.vertices);

    let adult = entity_model_mesh(&[EntityModelInstance::mooshroom(
        702,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    let (adult_min, adult_max) = mesh_extents(&adult);
    let (baby_min, baby_max) = mesh_extents(&baby);
    let adult_height = adult_max[1] - adult_min[1];
    let baby_height = baby_max[1] - baby_min[1];
    assert!(
        baby_height < adult_height,
        "baby mooshroom height {baby_height} should be smaller than adult {adult_height}"
    );
}

#[test]
fn mooshroom_renders_on_the_colored_path_unlike_the_graduated_cow() {
    // The mooshroom is a colored-only slice (the mushroom block-model layer and red/brown textures are
    // deferred), so it renders on both paths identically. The graduated cow, by contrast, is skipped on
    // the colored runtime path.
    let mooshroom_instances = [EntityModelInstance::mooshroom(
        703,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )];
    let full = entity_model_mesh(&mooshroom_instances);
    let colored = entity_model_colored_runtime_mesh(&mooshroom_instances);
    assert!(
        !colored.vertices.is_empty(),
        "the mooshroom renders colored"
    );
    assert_eq!(full.vertices, colored.vertices);
    assert_eq!(full.indices, colored.indices);

    let cow_colored = entity_model_colored_runtime_mesh(&[EntityModelInstance::cow(
        703,
        [0.0, 64.0, 0.0],
        0.0,
        false,
    )]);
    assert!(
        cow_colored.vertices.is_empty(),
        "the graduated cow is skipped on the colored runtime path"
    );
}

#[test]
fn mooshroom_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Mooshroom { baby: false }.model_key(),
        "mooshroom"
    );
    assert_eq!(
        EntityModelKind::Mooshroom { baby: true }.model_key(),
        "mooshroom_baby"
    );
}
