use super::*;

#[test]
fn item_frame_map_surface_uses_dynamic_map_texture_submission() {
    let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let light = [13.0 / 15.0, 1.0];
    let surface = bake_item_frame_map_surface(10, transform, light);

    assert_eq!(surface.vertex_count(), 4);
    assert_eq!(surface.index_count(), 6);
    assert_eq!(surface.submission.map_id, 10);
    assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
    assert_eq!(surface.submission.render_type.vanilla_name(), "text");
    assert_eq!(
        surface.submission.texture.vanilla_path(),
        "minecraft:map/10"
    );
    assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(surface.submission.transform, transform);
    assert_eq!(surface.submission.light, light);
    assert_eq!(
        (surface.submission.order, surface.submission.submit_sequence),
        (0, 0)
    );

    let first_map_rgba = vec![10; ITEM_FRAME_MAP_RGBA_LEN];
    let second_map_rgba = vec![20; ITEM_FRAME_MAP_RGBA_LEN];
    let (atlas, rgba) = build_item_frame_map_atlas(&[
        map_texture(10, second_map_rgba),
        map_texture(5, first_map_rgba),
        map_texture(99, vec![1, 2, 3]),
    ])
    .expect("valid map atlas");

    // The transient atlas is deterministic by map id and ignores malformed RGBA payloads.
    assert_eq!(atlas.width, ITEM_FRAME_MAP_SIZE);
    assert_eq!(atlas.height, ITEM_FRAME_MAP_SIZE * 2);
    assert_eq!(&rgba[..4], &[10, 10, 10, 10]);
    assert_eq!(
        &rgba[ITEM_FRAME_MAP_RGBA_LEN..ITEM_FRAME_MAP_RGBA_LEN + 4],
        &[20, 20, 20, 20]
    );
    assert!(!atlas.rects.contains_key(&99));

    let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);
    assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(vertices.len(), 4);
    assert_eq!(vertices[0].uv, [0.0, 1.0]);
    assert_eq!(vertices[2].uv, [1.0, 0.5]);
    assert!(vertices
        .iter()
        .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
}

#[test]
fn item_frame_map_surface_waits_when_dynamic_map_texture_is_absent() {
    let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    let (atlas, _) =
        build_item_frame_map_atlas(&[map_texture(5, vec![20; ITEM_FRAME_MAP_RGBA_LEN])])
            .expect("valid map atlas");

    let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

    assert!(
        vertices.is_empty() && indices.is_empty(),
        "absent dynamic map texture must not fold stale map geometry"
    );
}

#[test]
fn item_frame_map_surface_waits_when_dynamic_map_texture_is_malformed() {
    let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    let (atlas, _) = build_item_frame_map_atlas(&[
        map_texture(10, vec![1, 2, 3]),
        map_texture(5, vec![20; ITEM_FRAME_MAP_RGBA_LEN]),
    ])
    .expect("valid atlas from the unrelated map");

    let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

    assert!(
        vertices.is_empty() && indices.is_empty(),
        "malformed dynamic map texture must not fold stale map geometry"
    );
}

#[test]
fn persistent_map_owner_reuses_unchanged_pixels_and_uploads_only_one_patch() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
        return;
    };
    let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, [1.0; 2]);
    let second_surface = bake_item_frame_map_surface(11, Mat4::IDENTITY, [1.0; 2]);
    let first_rgba = vec![0; ITEM_FRAME_MAP_RGBA_LEN];
    let second_rgba = vec![9; ITEM_FRAME_MAP_RGBA_LEN];
    renderer.set_map_surfaces(
        vec![
            map_texture_at_revision(10, 7, 0, first_rgba.clone(), None),
            map_texture_at_revision(11, 8, 0, second_rgba.clone(), None),
        ],
        Vec::new(),
        vec![surface.clone(), second_surface.clone()],
        Vec::new(),
    );
    assert_eq!(renderer.counters().item_map_upload_bytes, 131_072);
    assert_eq!(renderer.counters().map_textures_created, 2);
    assert_eq!(renderer.counters().map_texture_resident_bytes, 131_072);
    assert_eq!(renderer.counters().map_surface_instances, 2);
    assert!(renderer.counters().map_instance_upload_bytes > 0);
    renderer
        .render(None)
        .expect("resident map instances render through the static quad pipeline");
    let slot = renderer
        .pack
        .item_frame_map_atlas
        .as_ref()
        .unwrap()
        .owners
        .get(&10)
        .unwrap()
        .slot;
    let second_owner = renderer
        .pack
        .item_frame_map_atlas
        .as_ref()
        .unwrap()
        .owners
        .get(&11)
        .unwrap();
    let second_slot = second_owner.slot;
    let second_owner_rgba = Arc::clone(&second_owner.rgba);

    renderer.set_map_surfaces(
        vec![
            map_texture_at_revision(10, 7, 0, first_rgba, None),
            map_texture_at_revision(11, 8, 0, second_rgba.clone(), None),
        ],
        Vec::new(),
        vec![surface.clone(), second_surface.clone()],
        Vec::new(),
    );
    assert_eq!(renderer.counters().item_map_upload_bytes, 0);
    assert_eq!(renderer.counters().map_textures_reused, 2);

    let pixel = [11, 22, 33, 255];
    let mut updated_rgba = vec![0; ITEM_FRAME_MAP_RGBA_LEN];
    let offset = (2 + 3 * ITEM_FRAME_MAP_SIZE as usize) * 4;
    updated_rgba[offset..offset + 4].copy_from_slice(&pixel);
    let replacements = renderer.counters().item_full_residency_replacements;
    renderer.set_map_surfaces(
        vec![
            map_texture_at_revision(
                10,
                7,
                1,
                updated_rgba,
                Some(ItemFrameMapTexturePatch {
                    start_x: 2,
                    start_y: 3,
                    width: 1,
                    height: 1,
                    rgba: Arc::from(pixel),
                }),
            ),
            map_texture_at_revision(11, 8, 0, second_rgba, None),
        ],
        Vec::new(),
        vec![surface, second_surface],
        Vec::new(),
    );
    assert_eq!(renderer.counters().item_map_upload_bytes, 4);
    assert_eq!(renderer.counters().map_textures_updated, 1);
    assert_eq!(
        renderer.counters().item_full_residency_replacements,
        replacements
    );
    let owner = renderer
        .pack
        .item_frame_map_atlas
        .as_ref()
        .unwrap()
        .owners
        .get(&10)
        .unwrap();
    assert_eq!(owner.slot, slot);
    assert_eq!(owner.content_revision, 1);
    let second_owner = renderer
        .pack
        .item_frame_map_atlas
        .as_ref()
        .unwrap()
        .owners
        .get(&11)
        .unwrap();
    assert_eq!(second_owner.slot, second_slot);
    assert_eq!(second_owner.content_revision, 0);
    assert!(Arc::ptr_eq(&second_owner.rgba, &second_owner_rgba));
}

#[test]
fn map_owner_capacity_falls_back_to_current_frame_page_without_hiding_surface() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer.set_map_texture_residency_limit_for_test(1);
    renderer.set_map_surfaces(
        vec![
            map_texture(10, vec![1; ITEM_FRAME_MAP_RGBA_LEN]),
            map_texture(11, vec![2; ITEM_FRAME_MAP_RGBA_LEN]),
        ],
        Vec::new(),
        vec![
            bake_item_frame_map_surface(10, Mat4::IDENTITY, [1.0; 2]),
            bake_item_frame_map_surface(11, Mat4::IDENTITY, [1.0; 2]),
        ],
        Vec::new(),
    );

    let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
    assert_eq!(atlas.owners.len(), 1);
    assert_eq!(atlas.overflow_locations.len(), 1);
    assert_eq!(renderer.pack.map_surface_residency.dynamic_draws.len(), 2);
    assert_eq!(renderer.counters().map_surface_instances, 2);
    assert_eq!(renderer.counters().map_texture_fallbacks, 1);
    assert_eq!(renderer.counters().item_map_upload_bytes, 131_072);
    assert_eq!(renderer.counters().map_full_residency_replacements, 0);
}

#[test]
fn overflow_map_pages_are_capped_and_reused_across_frames() {
    let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
        return;
    };
    renderer.set_map_surfaces(
        vec![map_texture(1, vec![1; ITEM_FRAME_MAP_RGBA_LEN])],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    {
        let atlas = renderer.pack.item_frame_map_atlas.as_mut().unwrap();
        atlas.max_resident_textures = 1;
        atlas.max_overflow_pages = 1;
    }
    let burst = (1..=(1 + MAP_ATLAS_PAGE_CAPACITY as i32 + 5))
        .map(|id| map_texture(id, vec![id as u8; ITEM_FRAME_MAP_RGBA_LEN]))
        .collect::<Vec<_>>();
    renderer.set_map_surfaces(burst.clone(), Vec::new(), Vec::new(), Vec::new());
    let first_page_ptr = {
        let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
        assert_eq!(atlas.overflow_pages.len(), 1);
        assert_eq!(atlas.overflow_locations.len(), MAP_ATLAS_PAGE_CAPACITY);
        &atlas.overflow_pages[0].texture as *const wgpu::Texture
    };

    // Second frame with the same burst reuses the existing overflow page in place.
    renderer.set_map_surfaces(burst, Vec::new(), Vec::new(), Vec::new());
    {
        let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
        assert_eq!(atlas.overflow_pages.len(), 1);
        assert_eq!(
            &atlas.overflow_pages[0].texture as *const wgpu::Texture,
            first_page_ptr
        );
    }

    // Returning to 1 resident map retires and clears the unused overflow page.
    renderer.set_map_surfaces(
        vec![map_texture(1, vec![1; ITEM_FRAME_MAP_RGBA_LEN])],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
    assert!(atlas.overflow_pages.is_empty());
    assert!(atlas.overflow_locations.is_empty());
}

#[test]
fn adding_second_map_page_uploads_only_new_owner_without_replacing_first_page() {
    let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
        return;
    };
    let first_page = (0..MAP_ATLAS_PAGE_CAPACITY as i32)
        .map(|map_id| map_texture(map_id, vec![map_id as u8; ITEM_FRAME_MAP_RGBA_LEN]))
        .collect::<Vec<_>>();
    renderer.set_map_surfaces(first_page.clone(), Vec::new(), Vec::new(), Vec::new());
    assert_eq!(
        renderer
            .pack
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .pages
            .len(),
        1
    );
    assert_eq!(renderer.counters().map_full_residency_replacements, 0);

    let mut second_page = first_page;
    second_page.push(map_texture(64, vec![64; ITEM_FRAME_MAP_RGBA_LEN]));
    renderer.set_map_surfaces(second_page, Vec::new(), Vec::new(), Vec::new());
    let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
    assert_eq!(atlas.pages.len(), 2);
    assert_eq!(atlas.owners.len(), MAP_ATLAS_PAGE_CAPACITY + 1);
    assert_eq!(renderer.counters().item_map_upload_bytes, 65_536);
    assert_eq!(renderer.counters().map_full_residency_replacements, 0);
}

#[test]
fn map_owner_remove_and_resource_invalidation_drop_old_handles() {
    let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
        return;
    };
    renderer.set_map_surfaces(
        vec![map_texture_at_revision(
            3,
            9,
            2,
            vec![1; ITEM_FRAME_MAP_RGBA_LEN],
            None,
        )],
        Vec::new(),
        vec![bake_item_frame_map_surface(3, Mat4::IDENTITY, [1.0; 2])],
        Vec::new(),
    );
    renderer.set_map_surfaces(
        vec![map_texture_at_revision(
            3,
            9,
            1,
            vec![2; ITEM_FRAME_MAP_RGBA_LEN],
            None,
        )],
        Vec::new(),
        vec![bake_item_frame_map_surface(3, Mat4::IDENTITY, [1.0; 2])],
        Vec::new(),
    );
    assert_eq!(renderer.counters().item_map_upload_bytes, 0);
    assert_eq!(renderer.counters().map_textures_stale, 1);
    assert_eq!(
        renderer
            .pack
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&3)
            .unwrap()
            .content_revision,
        2
    );
    renderer.remove_map_texture(3);
    let atlas = renderer.pack.item_frame_map_atlas.as_ref().unwrap();
    assert!(!atlas.owners.contains_key(&3));
    assert!(!atlas.layout.rects.contains_key(&3));

    renderer.invalidate_map_texture_residency();
    assert!(renderer.pack.item_frame_map_atlas.is_none());
    assert!(renderer.pack.first_person_map_background_atlas.is_none());
    assert!(renderer.pack.item_frame_map_decoration_atlas.is_none());
}

#[test]
fn first_person_map_background_surface_uses_vanilla_render_map_geometry() {
    let light = ITEM_MODEL_FULL_BRIGHT_LIGHT;
    let surface = bake_first_person_map_background_surface(
        FirstPersonMapBackgroundKind::Checkerboard,
        Mat4::IDENTITY,
        light,
    );

    assert_eq!(surface.vertex_count(), 4);
    assert_eq!(surface.index_count(), 6);
    assert_eq!(
        surface.submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
    assert_eq!(
        surface.submission.texture.vanilla_path(),
        "minecraft:textures/map/map_background_checkerboard.png"
    );
    assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(surface.submission.light, light);
    assert_eq!(
        surface
            .mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position)
            .collect::<Vec<_>>(),
        vec![
            [-7.0, 135.0, 0.0],
            [135.0, 135.0, 0.0],
            [135.0, -7.0, 0.0],
            [-7.0, -7.0, 0.0],
        ]
    );
}

#[test]
fn first_person_map_background_atlas_filters_and_remaps_uvs() {
    let (atlas, rgba) = build_first_person_map_background_atlas(&[
        FirstPersonMapBackgroundTexture {
            kind: FirstPersonMapBackgroundKind::Checkerboard,
            width: 1,
            height: 1,
            rgba: vec![40, 41, 42, 255],
        },
        FirstPersonMapBackgroundTexture {
            kind: FirstPersonMapBackgroundKind::Plain,
            width: 2,
            height: 1,
            rgba: vec![10, 11, 12, 255, 13, 14, 15, 255],
        },
        FirstPersonMapBackgroundTexture {
            kind: FirstPersonMapBackgroundKind::Plain,
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3],
        },
    ])
    .expect("valid map background atlas");

    assert_eq!(atlas.width, 2);
    assert_eq!(atlas.height, 2);
    assert_eq!(
        rgba,
        vec![10, 11, 12, 255, 13, 14, 15, 255, 40, 41, 42, 255, 0, 0, 0, 0,]
    );
    assert_eq!(atlas.rects.len(), 2);
    let surface = bake_first_person_map_background_surface(
        FirstPersonMapBackgroundKind::Checkerboard,
        Mat4::IDENTITY,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
    );
    let (vertices, indices) = merge_first_person_map_background_surfaces(&[surface], &atlas);
    assert_eq!(vertices.len(), 4);
    assert_eq!(indices.len(), 6);
    assert_eq!(vertices[0].uv, [0.0, 1.0]);
    assert_eq!(vertices[2].uv, [0.5, 0.5]);
}

#[test]
fn resource_map_atlases_persist_when_later_frames_carry_only_instances() {
    let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
        return;
    };
    let background = bake_first_person_map_background_surface(
        FirstPersonMapBackgroundKind::Checkerboard,
        Mat4::IDENTITY,
        [1.0; 2],
    );
    renderer.set_first_person_map_background_surfaces(
        vec![FirstPersonMapBackgroundTexture {
            kind: FirstPersonMapBackgroundKind::Checkerboard,
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        }],
        vec![background.clone()],
    );
    renderer.set_first_person_map_background_surfaces(Vec::new(), vec![background]);
    assert!(renderer.pack.first_person_map_background_atlas.is_some());
    assert_eq!(renderer.first_person_map_background_surfaces.len(), 1);

    let decoration =
        bake_first_person_map_decoration_surface(1, 0, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0)
            .unwrap();
    let sprite_id = decoration.submission.texture.sprite_id.to_string();
    renderer.set_map_decoration_surfaces(
        Vec::new(),
        vec![ItemFrameMapDecorationTexture {
            sprite_id,
            width: 1,
            height: 1,
            rgba: vec![1, 2, 3, 255],
        }],
        Vec::new(),
        vec![decoration.clone()],
    );
    renderer.set_map_decoration_surfaces(Vec::new(), Vec::new(), Vec::new(), vec![decoration]);
    assert!(renderer.pack.item_frame_map_decoration_atlas.is_some());
    assert_eq!(renderer.first_person_map_decoration_surfaces.len(), 1);
}

#[test]
fn item_frame_map_decoration_surface_uses_vanilla_text_submission() {
    let map_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let light = [7.0 / 15.0, 12.0 / 15.0];
    let surface = bake_item_frame_map_decoration_surface(1, -20, 30, 7, 2, map_transform, light, 3)
        .expect("frame decorations render on item-frame maps");

    let expected_transform = map_transform
        * Mat4::from_translation(Vec3::new(-20.0 / 2.0 + 64.0, 30.0 / 2.0 + 64.0, -0.02))
        * Mat4::from_rotation_z((7.0_f32 * 360.0 / 16.0).to_radians())
        * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
        * Mat4::from_translation(Vec3::new(-0.125, 0.125, 0.0));

    assert_eq!(surface.vertex_count(), 4);
    assert_eq!(surface.index_count(), 6);
    assert_eq!(surface.submission.type_id, 1);
    assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
    assert_eq!(surface.submission.render_type.vanilla_name(), "text");
    assert_eq!(
        surface.submission.texture.vanilla_atlas_path(),
        "minecraft:textures/atlas/map_decorations.png"
    );
    assert_eq!(
        surface.submission.texture.vanilla_sprite_id(),
        "minecraft:frame"
    );
    assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(surface.submission.transform, expected_transform);
    assert_eq!(surface.submission.light, light);
    assert_eq!(
        (surface.submission.order, surface.submission.submit_sequence),
        (0, 3)
    );
    assert_eq!(surface.submission.decoration_index, 2);
    assert_eq!(
        surface.mesh.vertices[0].position,
        expected_transform
            .transform_point3(Vec3::new(-1.0, 1.0, -0.002))
            .to_array()
    );
    assert!(surface
        .mesh
        .vertices
        .iter()
        .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
    assert!(
        bake_item_frame_map_decoration_surface(0, 0, 0, 0, 0, map_transform, light, 1).is_none(),
        "player marker has showOnItemFrame=false in vanilla"
    );
    let first_person =
        bake_first_person_map_decoration_surface(0, 0, 0, 0, 0, map_transform, light, 1)
            .expect("first-person maps render all MapRenderer decorations");
    assert_eq!(first_person.submission.type_id, 0);
    assert_eq!(
        first_person.submission.texture.vanilla_sprite_id(),
        "minecraft:player"
    );
}

#[test]
fn item_frame_map_text_surface_uses_vanilla_order_one_text_submission() {
    let glyphs = test_map_text_glyphs();
    let map_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
    let light = [7.0 / 15.0, 12.0 / 15.0];
    let surface = plain_bake_item_frame_map_text_surface(
        1,
        "Frame",
        -20,
        30,
        0,
        map_transform,
        light,
        0,
        &glyphs,
    )
    .expect("frame-visible decoration name text");

    let width = 30.0;
    let scale = 6.0 / 9.0;
    let expected_transform = map_transform
        * Mat4::from_translation(Vec3::new(
            -20.0 / 2.0 + 64.0 - width * scale / 2.0,
            30.0 / 2.0 + 64.0 + 4.0,
            -0.025,
        ))
        * Mat4::from_scale(Vec3::new(scale, scale, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));

    assert_eq!(surface.vertex_count(), 24);
    assert_eq!(surface.index_count(), 36);
    assert_eq!(surface.submission.type_id, 1);
    assert_eq!(surface.submission.text, "Frame");
    assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
    assert_eq!(surface.submission.render_type.vanilla_name(), "text");
    assert_eq!(
        surface.submission.texture.vanilla_path(),
        "minecraft:font/default"
    );
    assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(surface.submission.transform, expected_transform);
    assert_eq!(surface.submission.light, light);
    assert_eq!(
        (surface.submission.order, surface.submission.submit_sequence),
        (1, 0)
    );
    assert_eq!(surface.submission.decoration_index, 0);
    assert_eq!(surface.submission.width, width);
    assert_eq!(surface.submission.scale, scale);
    assert_eq!(
        surface.mesh.vertices[0].position,
        expected_transform
            .transform_point3(Vec3::new(0.0, 0.0, 0.0))
            .to_array()
    );
    assert!(surface
        .mesh
        .vertices
        .iter()
        .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
    assert!(
        plain_bake_item_frame_map_text_surface(
            0,
            "Hidden",
            0,
            0,
            0,
            map_transform,
            light,
            0,
            &glyphs
        )
        .is_none(),
        "player marker has showOnItemFrame=false in vanilla"
    );
    let first_person = plain_bake_first_person_map_text_surface(
        0,
        "Player",
        0,
        0,
        0,
        map_transform,
        light,
        0,
        &glyphs,
    )
    .expect("first-person maps render text for non-frame decorations");
    assert_eq!(first_person.submission.type_id, 0);
    assert_eq!(first_person.submission.text, "Player");
}

#[test]
fn map_text_uses_resident_quad_and_narrow_glyph_instances() {
    let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
        return;
    };
    renderer
        .upload_hud_font_atlas(
            1,
            1,
            &[255, 255, 255, 255],
            renderer.pack.hud_font_book.clone(),
        )
        .unwrap();
    let surface = plain_bake_item_frame_map_text_surface(
        1,
        "Frame",
        0,
        0,
        0,
        Mat4::IDENTITY,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
        0,
        &test_map_text_glyphs(),
    )
    .expect("map text");
    let instance_count = surface.glyphs.len() + 1;
    renderer.set_map_text_surfaces(vec![surface], Vec::new());

    assert_eq!(
        renderer.pack.map_surface_residency.text_instances.len(),
        instance_count
    );
    assert_eq!(renderer.pack.map_surface_residency.text_draws.len(), 1);
    assert_eq!(
        renderer.pack.map_surface_residency.text_draws[0].target,
        MapSurfaceTarget::World
    );
    assert_eq!(renderer.counters().map_surface_instances, instance_count);
    assert_eq!(
        renderer.counters().map_instance_upload_bytes,
        (instance_count * std::mem::size_of::<MapSurfaceGpuInstance>()) as u64
    );
    renderer.update_camera();
    renderer.render_offscreen_frame().unwrap();
}

#[test]
fn item_frame_map_text_width_uses_glyph_map_replacement_fallback() {
    let mut glyphs = test_map_text_glyphs();
    glyphs.insert_first_wins(
        'é',
        HudAsciiGlyph {
            width: 6,
            height: 12,
            advance: 7.0,
            bearing_top: 10.0,
            ..HudAsciiGlyph::default()
        },
    );
    assert_eq!(plain_item_frame_map_text_width("A A", &glyphs), Some(16));
    // Mapped accented codepoints resolve their own glyph now.
    assert_eq!(plain_item_frame_map_text_width("é", &glyphs), Some(7));
    // This fixture deliberately has no unihex provider, so CJK is `?`.
    assert_eq!(plain_item_frame_map_text_width("钻", &glyphs), Some(5));
    assert_eq!(plain_item_frame_map_text_width("", &glyphs), Some(0));
}

#[test]
fn map_text_positions_from_logical_width_and_draws_visual_glyph_order() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, min_x) in [('1', 0.1), ('2', 0.2), ('3', 0.3), ('-', 0.4), ('?', 0.5)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                uv: crate::HudUvRect {
                    min: [min_x, 0.0],
                    max: [min_x + 0.05, 0.5],
                },
                width: 5,
                height: 8,
                advance: 6.0,
                bearing_top: 7.0,
                ..HudAsciiGlyph::default()
            },
        );
    }
    glyphs.set_default_right_to_left(true);

    let surface = plain_bake_item_frame_map_text_surface(
        1,
        "123-",
        0,
        0,
        0,
        Mat4::IDENTITY,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
        0,
        &glyphs,
    )
    .unwrap();
    assert_eq!(surface.submission.text, "123-");
    assert_eq!(surface.submission.width, 24.0);
    let glyph_uvs = surface
        .glyphs
        .iter()
        .map(|glyph| glyph.uv.min[0])
        .collect::<Vec<_>>();
    assert_eq!(glyph_uvs, [0.4, 0.1, 0.2, 0.3]);
}

#[test]
fn item_frame_map_decoration_atlas_remaps_sprite_uvs() {
    let (atlas, rgba) = build_item_frame_map_decoration_atlas(&[
        ItemFrameMapDecorationTexture {
            sprite_id: "minecraft:target_x".to_string(),
            width: 1,
            height: 1,
            rgba: vec![20, 21, 22, 255],
        },
        ItemFrameMapDecorationTexture {
            sprite_id: "minecraft:frame".to_string(),
            width: 2,
            height: 1,
            rgba: vec![10, 11, 12, 255, 13, 14, 15, 255],
        },
        ItemFrameMapDecorationTexture {
            sprite_id: "minecraft:bad".to_string(),
            width: 2,
            height: 2,
            rgba: vec![1, 2, 3],
        },
    ])
    .expect("valid map decoration atlas");

    assert_eq!(atlas.width, 2);
    assert_eq!(atlas.height, 2);
    assert_eq!(&rgba[0..8], &[10, 11, 12, 255, 13, 14, 15, 255]);
    assert_eq!(&rgba[8..16], &[20, 21, 22, 255, 0, 0, 0, 0]);
    assert!(!atlas.rects.contains_key("minecraft:bad"));

    let surface = bake_item_frame_map_decoration_surface(
        1,
        0,
        0,
        0,
        0,
        Mat4::IDENTITY,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
        1,
    )
    .expect("frame decoration surface");
    let (vertices, indices) = merge_item_frame_map_decoration_surfaces(&[surface], &atlas);
    assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
    assert_eq!(vertices.len(), 4);
    assert_eq!(vertices[0].uv, [0.0, 0.0]);
    assert_eq!(vertices[2].uv, [1.0, 0.5]);
}

#[test]
fn fractional_map_text_ceil_and_negative_scale_boundary() {
    let mut glyphs = HudFontGlyphMap::new();
    for (ch, advance) in [('a', 0.5), ('b', -3.0)] {
        glyphs.insert_first_wins(
            ch,
            HudAsciiGlyph {
                advance,
                width: 1,
                height: 8,
                ..HudAsciiGlyph::default()
            },
        );
    }
    for (text, width, scale) in [
        ("aa", 1, 6.0 / 9.0),
        ("ba", -2, 0.0),
        ("aaaaaab", 0, 6.0 / 9.0),
    ] {
        assert_eq!(plain_item_frame_map_text_width(text, &glyphs), Some(width));
        let surface = plain_bake_item_frame_map_text_surface(
            1,
            text,
            0,
            0,
            0,
            Mat4::IDENTITY,
            [0.0, 1.0],
            0,
            &glyphs,
        )
        .unwrap();
        assert_eq!(surface.submission.width, width as f32);
        assert_eq!(surface.submission.scale, scale);
        assert!(
            (surface.glyphs[0].transform.w_axis.x - (64.0 - width as f32 * scale / 2.0)).abs()
                < 0.00001
        );
        let delta = surface.glyphs[1].transform.w_axis.x - surface.glyphs[0].transform.w_axis.x;
        assert!((delta - if text.starts_with('b') { -3.0 } else { 0.5 } * scale).abs() < 0.00001);
    }
}

#[test]
fn map_text_uses_missing_atlas_when_no_codepoints_are_registered() {
    let mut glyphs = HudFontGlyphMap::new();
    let missing = HudAsciiGlyph {
        width: 5,
        height: 8,
        advance: 6.0,
        ..Default::default()
    };
    glyphs.set_missing_glyph(missing);
    assert_eq!(plain_item_frame_map_text_width("缺", &glyphs), Some(6));
    let surface = plain_bake_item_frame_map_text_surface(
        1,
        "缺",
        0,
        0,
        0,
        Mat4::IDENTITY,
        ITEM_MODEL_FULL_BRIGHT_LIGHT,
        0,
        &glyphs,
    )
    .unwrap();
    assert_eq!(surface.submission.width, 6.0);
    assert_eq!(surface.glyphs.len(), 1);
    assert_eq!(surface.mesh.vertices[0].uv, missing.uv.min);
}

#[test]
fn bitmap_map_text_preserves_signed_fractional_glyph_geometry() {
    for render_scale in [3.0 / 7.0, -3.0 / 7.0, 0.0] {
        let mut glyphs = HudFontGlyphMap::new();
        let glyph = HudAsciiGlyph {
            width: 5,
            height: 7,
            render_scale,
            bearing_top: -3.0,
            advance: 3.0,
            ..Default::default()
        };
        glyphs.insert_first_wins('A', glyph);
        let surface = plain_bake_item_frame_map_text_surface(
            1,
            "A",
            0,
            0,
            0,
            Mat4::IDENTITY,
            [0.0, 1.0],
            0,
            &glyphs,
        )
        .unwrap();
        let instance = &surface.glyphs[0];
        let scale = 6.0 / 9.0;
        assert_eq!(instance.transform.x_axis.x, glyph.rendered_width() * scale);
        assert_eq!(instance.transform.y_axis.y, glyph.rendered_height() * scale);
        assert!((instance.transform.w_axis.y - (68.0 + 10.0 * scale)).abs() < 0.00001);
    }
}

#[test]
fn map_text_zero_bitmap_submits_and_empty_only_advances() {
    for render_scale in [1.0, f32::INFINITY] {
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            'a',
            HudAsciiGlyph {
                advance: 3.0,
                render_scale,
                ..Default::default()
            },
        );
        glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(3.0));
        let surface = plain_bake_item_frame_map_text_surface(
            1,
            "a a",
            0,
            0,
            0,
            Mat4::IDENTITY,
            [0.0, 1.0],
            0,
            &glyphs,
        )
        .unwrap();
        assert_eq!(surface.glyphs.len(), 2);
        assert_eq!(surface.mesh.vertices.len(), 8);
        assert_eq!(surface.submission.width, 9.0);
        if render_scale.is_infinite() {
            assert!(!surface.glyphs[0].transform.is_finite());
            assert!(surface.mesh.vertices[2].position.iter().any(|x| x.is_nan()));
        } else {
            assert!(
                (surface.glyphs[1].transform.w_axis.x - surface.glyphs[0].transform.w_axis.x - 4.0)
                    .abs()
                    < 0.00001
            );
            assert_eq!(surface.glyphs[0].transform.x_axis.x, 0.0);
        }
    }
}

fn plain_item_frame_map_text_width(text: &str, glyphs: &HudFontGlyphMap) -> Option<i32> {
    item_frame_map_text_width(&[WorldTextRun::plain(text)], &map_text_test_fonts(glyphs))
}

fn map_text_test_fonts(glyphs: &HudFontGlyphMap) -> HudFontGlyphBook {
    HudFontGlyphBook::from_fonts(
        "minecraft:default",
        BTreeMap::from([("minecraft:default".into(), glyphs.clone())]),
    )
}

#[allow(clippy::too_many_arguments)]
fn plain_bake_item_frame_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    index: u32,
    transform: Mat4,
    light: [f32; 2],
    sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_item_frame_map_text_surface(
        type_id,
        &[WorldTextRun::plain(text)],
        x,
        y,
        index,
        transform,
        light,
        sequence,
        &map_text_test_fonts(glyphs),
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn plain_bake_first_person_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    index: u32,
    transform: Mat4,
    light: [f32; 2],
    sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_first_person_map_text_surface(
        type_id,
        &[WorldTextRun::plain(text)],
        x,
        y,
        index,
        transform,
        light,
        sequence,
        &map_text_test_fonts(glyphs),
        0,
    )
}
