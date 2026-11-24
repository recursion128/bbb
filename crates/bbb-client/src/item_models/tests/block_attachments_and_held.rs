use super::*;

#[test]
fn fast_leaves_moving_blocks_select_solid_without_alpha_cutout() {
    let cutout_leaves = TerrainTextureState::default();
    let fast_leaves = TerrainTextureState::default().with_cutout_leaves(false);

    assert_eq!(
        block_model_solid_phase(true, "minecraft:oak_leaves", &cutout_leaves),
        ItemModelDrawPhase::Solid
    );
    assert_eq!(
        block_model_solid_phase(true, "minecraft:oak_leaves", &fast_leaves),
        ItemModelDrawPhase::SolidNoCutout
    );
    assert_eq!(
        block_model_solid_phase(true, "minecraft:stone", &fast_leaves),
        ItemModelDrawPhase::Solid
    );
    assert_eq!(
        block_model_solid_phase(false, "minecraft:oak_leaves", &fast_leaves),
        ItemModelDrawPhase::Solid
    );
}

#[test]
fn brown_mooshroom_uses_the_brown_mushroom_attachment() {
    assert_eq!(
        mooshroom_mushroom_block_id(MooshroomVariant::Brown),
        BROWN_MUSHROOM_BLOCK_ID
    );
}

#[test]
fn entity_block_attachments_collect_snow_golem_iron_golem_and_mooshroom_layers() {
    let world = WorldStore::new();
    let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.snow_golem_mut().pumpkin = true)
        .with_outline_color(0xff55_aa11);
    let iron_golem = EntityModelInstance::iron_golem(74, [1.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.iron_golem_mut().offer_flower_tick = 400)
        .with_outline_color(0xff55_aa11);
    let idle_iron_golem = EntityModelInstance::iron_golem(75, [2.0, 64.0, 0.0], 0.0);
    let mooshroom = EntityModelInstance::mooshroom(86, [3.0, 64.0, 0.0], 0.0, false)
        .with_outline_color(0xff55_aa11);
    let baby_mooshroom = EntityModelInstance::mooshroom(87, [4.0, 64.0, 0.0], 0.0, true);

    let attachments = entity_block_attachments(
        &[
            snow_golem,
            iron_golem,
            idle_iron_golem,
            mooshroom,
            baby_mooshroom,
        ],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );

    assert_eq!(attachments.len(), 5);
    assert!(attachments
        .iter()
        .all(|attachment| !attachment.outline_only));
    assert!(attachments
        .iter()
        .all(|attachment| attachment.outline_color == 0xff55_aa11));
    assert_eq!(attachments[0].block_id.as_ref(), CARVED_PUMPKIN_BLOCK_ID);
    assert_eq!(
        attachments[0].properties.as_ref(),
        &carved_pumpkin_default_properties()
    );
    assert_eq!(attachments[1].block_id.as_ref(), POPPY_BLOCK_ID);
    assert_eq!(
        attachments[1].properties.as_ref(),
        &poppy_default_properties()
    );
    for attachment in &attachments[2..] {
        assert_eq!(attachment.block_id.as_ref(), RED_MUSHROOM_BLOCK_ID);
        assert_eq!(
            attachment.properties.as_ref(),
            &mooshroom_mushroom_default_properties()
        );
    }
}

#[test]
fn entity_block_attachments_record_invisible_glowing_outline_only_layers() {
    let world = WorldStore::new();
    let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.snow_golem_mut().pumpkin = true)
        .with_invisible(true)
        .with_outline_color(0xff55_aa11);
    let mooshroom = EntityModelInstance::mooshroom(86, [3.0, 64.0, 0.0], 0.0, false)
        .with_invisible(true)
        .with_outline_color(0xff55_aa11);

    let attachments = entity_block_attachments(
        &[snow_golem, mooshroom],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );

    assert_eq!(attachments.len(), 4);
    assert!(attachments.iter().all(|attachment| attachment.outline_only));
    assert!(attachments
        .iter()
        .all(|attachment| attachment.outline_color == 0xff55_aa11));
    assert_eq!(attachments[0].block_id.as_ref(), CARVED_PUMPKIN_BLOCK_ID);
    assert_eq!(
        attachments[0].properties.as_ref(),
        &carved_pumpkin_default_properties()
    );
    for attachment in &attachments[1..] {
        assert_eq!(attachment.block_id.as_ref(), RED_MUSHROOM_BLOCK_ID);
        assert_eq!(
            attachment.properties.as_ref(),
            &mooshroom_mushroom_default_properties()
        );
    }
}

#[test]
fn entity_block_models_retain_outline_only_solid_and_translucent_geometry() {
    let world = WorldStore::new();
    let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.snow_golem_mut().pumpkin = true)
        .with_invisible(true)
        .with_outline_color(0xff55_aa11);
    let terrain_textures = TerrainTextureState::default();
    let mut quads = unit_block_quads();
    let mut translucent = quads[0];
    translucent.translucent = true;
    quads.push(translucent);
    terrain_textures.insert_block_item_quads_for_test(
        CARVED_PUMPKIN_BLOCK_ID,
        carved_pumpkin_default_properties(),
        Arc::from(quads),
    );

    let residency = entity_block_models(
        &[snow_golem],
        &world,
        None,
        &terrain_textures,
        1.0,
        MovingPistonVisibility::default(),
        false,
    );

    assert_eq!(residency.assets.len(), 2);
    assert_eq!(residency.instances.len(), 2);
    assert!(residency
        .instances
        .iter()
        .all(|instance| instance.target == ItemModelDrawTarget::EntityOutline));
    assert!(residency
        .instances
        .iter()
        .all(|instance| instance.phase == ItemModelDrawPhase::Outline));
    assert!(residency.instances.iter().all(|instance| {
        instance.tint
            == [
                0x55 as f32 / 255.0,
                0xaa as f32 / 255.0,
                0x11 as f32 / 255.0,
                1.0,
            ]
    }));
    assert_eq!(
        residency
            .instances
            .iter()
            .map(|instance| instance.geometry_class)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ItemModelGeometryClass::Solid,
            ItemModelGeometryClass::Translucent,
        ])
    );
    assert!(!residency
        .instances
        .iter()
        .any(|instance| instance.target == ItemModelDrawTarget::BlockFeature));
}

#[test]
fn entity_block_models_reuse_visible_attachment_geometry_for_main_and_outline() {
    let world = WorldStore::new();
    let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.snow_golem_mut().pumpkin = true)
        .with_outline_color(0xff55_aa11);
    let terrain_textures = TerrainTextureState::default();
    let mut quads = unit_block_quads();
    let mut translucent = quads[0];
    translucent.translucent = true;
    quads.push(translucent);
    terrain_textures.insert_block_item_quads_for_test(
        CARVED_PUMPKIN_BLOCK_ID,
        carved_pumpkin_default_properties(),
        Arc::from(quads),
    );

    let residency = entity_block_models(
        &[snow_golem],
        &world,
        None,
        &terrain_textures,
        1.0,
        MovingPistonVisibility::default(),
        false,
    );

    assert_eq!(
        residency.assets.len(),
        2,
        "solid and translucent classes each have one static geometry request"
    );
    assert_eq!(residency.instances.len(), 4);
    for pair in residency.instances.as_chunks::<2>().0 {
        let main = &pair[0];
        let outline = &pair[1];
        assert_eq!(main.identity, outline.identity);
        assert_eq!(main.atlas, outline.atlas);
        assert_eq!(main.geometry_class, outline.geometry_class);
        assert_eq!(main.transform, outline.transform);
        assert_eq!(main.target, ItemModelDrawTarget::BlockFeature);
        assert_eq!(
            main.phase,
            match main.geometry_class {
                ItemModelGeometryClass::Solid => ItemModelDrawPhase::Solid,
                ItemModelGeometryClass::Translucent => ItemModelDrawPhase::Translucent,
            }
        );
        assert_eq!(main.tint, [1.0; 4]);
        assert_eq!(outline.target, ItemModelDrawTarget::EntityOutline);
        assert_eq!(outline.phase, ItemModelDrawPhase::Outline);
        assert_eq!(
            outline.tint,
            [
                0x55 as f32 / 255.0,
                0xaa as f32 / 255.0,
                0x11 as f32 / 255.0,
                1.0,
            ]
        );
        assert_eq!(main.light, outline.light);
        assert_eq!(main.overlay, outline.overlay);
        assert!(main.submit_order < outline.submit_order);
    }
}

#[test]
fn entity_block_models_emit_no_outline_copy_for_zero_color() {
    let world = WorldStore::new();
    let snow_golem = EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.kind.snow_golem_mut().pumpkin = true);
    let terrain_textures = TerrainTextureState::default();
    terrain_textures.insert_block_item_quads_for_test(
        CARVED_PUMPKIN_BLOCK_ID,
        carved_pumpkin_default_properties(),
        Arc::from(unit_block_quads()),
    );

    let residency = entity_block_models(
        &[snow_golem],
        &world,
        None,
        &terrain_textures,
        1.0,
        MovingPistonVisibility::default(),
        false,
    );

    assert_eq!(residency.assets.len(), 1);
    assert_eq!(residency.instances.len(), 1);
    assert_eq!(
        residency.instances[0].target,
        ItemModelDrawTarget::BlockFeature
    );
    assert_eq!(residency.instances[0].phase, ItemModelDrawPhase::Solid);
}

#[test]
fn entity_block_attachments_collect_enderman_carried_block_from_world() {
    let entity_id = 41;
    let world = world_with_enderman_carried_grass_block(entity_id);
    let light_coords = (6_u32 << 4) | (10_u32 << 20);
    let enderman = EntityModelInstance::enderman(entity_id, [0.0, 64.0, 0.0], 0.0)
        .with(|s| (s.kind.enderman_mut().carrying, s.light_coords) = (true, light_coords))
        .with_outline_color(0xff55_aa11);

    let attachments = entity_block_attachments(
        &[enderman],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
    assert_eq!(
        attachments[0].properties.as_ref(),
        &BTreeMap::from([("snowy".to_string(), "false".to_string())])
    );
    assert_eq!(attachments[0].light, [6.0 / 15.0, 10.0 / 15.0]);
    assert_eq!(attachments[0].outline_color, 0xff55_aa11);
    assert!(!attachments[0].outline_only);
}

#[test]
fn entity_block_attachments_collect_minecart_display_block_from_world() {
    let entity_id = 85;
    let world = world_with_chest_minecart(entity_id);
    let light_coords = (6_u32 << 4) | (10_u32 << 20);
    let minecart = EntityModelInstance::minecart(entity_id, [0.0, 64.0, 0.0], 45.0)
        .with(|s| s.light_coords = light_coords);

    let attachments = entity_block_attachments(
        &[minecart],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:chest");
    assert_eq!(
        attachments[0].properties.as_ref(),
        &BTreeMap::from([
            ("facing".to_string(), "north".to_string()),
            ("type".to_string(), "single".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ])
    );
    assert_eq!(attachments[0].light, [6.0 / 15.0, 10.0 / 15.0]);
    assert_eq!(attachments[0].overlay, ITEM_MODEL_NO_OVERLAY);
    assert!(!attachments[0].outline_only);
    assert!(attachments[0]
        .transform
        .transform_point3(Vec3::splat(0.5))
        .is_finite());

    let glowing = minecart.with_outline_color(0xff55_aa11);
    let glowing_attachments = entity_block_attachments(
        &[glowing],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );
    assert_eq!(glowing_attachments[0].outline_color, 0xff55_aa11);
    assert!(!glowing_attachments[0].outline_only);

    let hidden = minecart.with_invisible(true);
    assert!(entity_block_attachments(
        &[hidden],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default()
    )
    .is_empty());

    let outline = hidden.with_appears_glowing(true);
    let outline_attachments = entity_block_attachments(
        &[outline],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );
    assert_eq!(outline_attachments.len(), 1);
    assert!(
        !outline_attachments[0].outline_only,
        "AbstractMinecartRenderer always uses ordinary BlockModelRenderState.submit"
    );
}

#[test]
fn entity_block_attachments_apply_tnt_minecart_fuse_scale_and_white_overlay() {
    let entity_id = 133;
    let world = world_with_tnt_minecart(entity_id);
    let light_coords = (5_u32 << 4) | (12_u32 << 20);
    let primed = EntityModelInstance::minecart(entity_id, [0.0, 64.0, 0.0], 0.0).with(|s| {
        (
            s.light_coords,
            s.kind.minecart_mut().tnt_fuse_remaining_in_ticks,
        ) = (light_coords, 4.5)
    });

    let attachments = entity_block_attachments(
        &[primed],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:tnt");
    assert_eq!(
        attachments[0].properties.as_ref(),
        &BTreeMap::from([("unstable".to_string(), "false".to_string())])
    );
    assert_eq!(attachments[0].light, [5.0 / 15.0, 12.0 / 15.0]);
    assert_eq!(attachments[0].overlay, [15.0, 10.0]);
    assert_eq!(attachments[0].outline_color, 0);
    assert!(!attachments[0].outline_only);
    assert_eq!(
        attachments[0].transform,
        minecart_tnt_display_block_transform(&primed, 6).unwrap()
    );

    let unprimed = primed.with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = -1.0);
    let unprimed_attachment = entity_block_attachments(
        &[unprimed],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    )
    .pop()
    .expect("unprimed tnt attachment");
    assert_eq!(unprimed_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
    assert_ne!(unprimed_attachment.transform, attachments[0].transform);

    let odd_strobe = primed.with(|s| s.kind.minecart_mut().tnt_fuse_remaining_in_ticks = 6.0);
    let odd_strobe_attachment = entity_block_attachments(
        &[odd_strobe],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    )
    .pop()
    .expect("odd strobe tnt attachment");
    assert_eq!(odd_strobe_attachment.overlay, ITEM_MODEL_NO_OVERLAY);

    let glowing = primed.with_outline_color(0xff55_aa11);
    let glowing_attachment = entity_block_attachments(
        &[glowing],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    )
    .pop()
    .expect("glowing tnt minecart attachment");
    assert_eq!(glowing_attachment.outline_color, 0xff55_aa11);
    assert!(!glowing_attachment.outline_only);
}

#[test]
fn entity_block_attachments_apply_primed_tnt_block_state_fuse_and_outline() {
    let entity_id = 134;
    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let world = world_with_primed_tnt(entity_id, 4, Some(GRASS_BLOCK_STATE_ID));
    let light_coords = (5_u32 << 4) | (12_u32 << 20);
    let tnt = EntityModelInstance::no_render(entity_id, [0.0, 64.0, 0.0], 0.0)
        .with(|s| s.light_coords = light_coords);

    let attachments =
        entity_block_attachments(&[tnt], &world, None, 0.5, MovingPistonVisibility::default());

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
    assert_eq!(attachments[0].properties.as_ref(), &grass_props);
    assert_eq!(attachments[0].light, [5.0 / 15.0, 12.0 / 15.0]);
    assert_eq!(attachments[0].overlay, [15.0, 10.0]);
    assert_eq!(
        attachments[0].transform,
        primed_tnt_block_transform(&tnt, 4.5).unwrap()
    );
    assert_eq!(attachments[0].outline_color, 0);
    assert!(!attachments[0].outline_only);
    let hidden = tnt.with_invisible(true);
    assert!(entity_block_attachments(
        &[hidden],
        &world,
        None,
        0.5,
        MovingPistonVisibility::default()
    )
    .is_empty());
    let outline = hidden.with_appears_glowing(true);
    let outline_attachments = entity_block_attachments(
        &[outline],
        &world,
        None,
        0.5,
        MovingPistonVisibility::default(),
    );
    assert_eq!(outline_attachments.len(), 1);
    assert!(
        !outline_attachments[0].outline_only,
        "TntRenderer always uses ordinary BlockModelRenderState.submit"
    );
    assert_ne!(outline_attachments[0].outline_color, 0);

    let default_world = world_with_primed_tnt(entity_id, 6, None);
    let default_attachment = entity_block_attachments(
        &[tnt],
        &default_world,
        None,
        0.0,
        MovingPistonVisibility::default(),
    )
    .pop()
    .expect("default primed tnt attachment");
    assert_eq!(default_attachment.block_id.as_ref(), "minecraft:tnt");
    assert_eq!(
        default_attachment.properties.as_ref(),
        &BTreeMap::from([("unstable".to_string(), "false".to_string())])
    );
    assert_eq!(default_attachment.overlay, ITEM_MODEL_NO_OVERLAY);
}

#[test]
fn entity_block_attachments_apply_falling_block_state_and_should_render_gate() {
    let entity_id = 135;
    let grass_props = BTreeMap::from([("snowy".to_string(), "false".to_string())]);
    let mut world = world_with_falling_block(entity_id, GRASS_BLOCK_STATE_ID, [1.5, 4.0, 2.5]);
    let light_coords = (7_u32 << 4) | (11_u32 << 20);
    let falling = EntityModelInstance::no_render(entity_id, [1.5, 4.0, 2.5], 90.0)
        .with(|s| s.light_coords = light_coords);

    let attachments = entity_block_attachments(
        &[falling],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );

    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].block_id.as_ref(), "minecraft:grass_block");
    assert_eq!(attachments[0].properties.as_ref(), &grass_props);
    assert_eq!(attachments[0].light, [7.0 / 15.0, 11.0 / 15.0]);
    assert_eq!(attachments[0].overlay, ITEM_MODEL_NO_OVERLAY);
    assert_eq!(
        attachments[0].transform,
        falling_block_transform(&falling).unwrap()
    );
    assert!(!attachments[0].outline_only);
    assert_eq!(
        attachments[0].moving_block,
        Some(MovingBlockRenderContext {
            random_seed_pos: BlockPos { x: 1, y: 4, z: 2 },
            block_pos: BlockPos { x: 1, y: 4, z: 2 },
            biome_id: None,
            cardinal_lighting: TerrainCardinalLighting::Default,
        })
    );

    let hidden_outline = falling.with_invisible(true).with_appears_glowing(true);
    let hidden_attachments = entity_block_attachments(
        &[hidden_outline],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );
    assert_eq!(hidden_attachments.len(), 1);
    assert!(
        !hidden_attachments[0].outline_only,
        "vanilla FallingBlockRenderer.submit does not gate the body on isInvisible"
    );

    assert!(falling_block_should_render(
        &falling,
        &world,
        GRASS_BLOCK_STATE_ID
    ));
    insert_entity_block_test_chunk(&mut world);
    let feet = BlockPos { x: 1, y: 4, z: 2 };
    set_entity_block_test_block(&mut world, feet, GRASS_BLOCK_STATE_ID);
    assert!(!falling_block_should_render(
        &falling,
        &world,
        GRASS_BLOCK_STATE_ID
    ));
    assert!(entity_block_attachments(
        &[falling],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default()
    )
    .is_empty());

    set_entity_block_test_block(
        &mut world,
        feet,
        bbb_client_world::vanilla_air_block_state_id(),
    );
    assert!(falling_block_should_render(
        &falling,
        &world,
        GRASS_BLOCK_STATE_ID
    ));
    assert_eq!(
        entity_block_attachments(
            &[falling],
            &world,
            None,
            1.0,
            MovingPistonVisibility::default()
        )
        .len(),
        1
    );
    let loaded = entity_block_attachments(
        &[falling],
        &world,
        None,
        1.0,
        MovingPistonVisibility::default(),
    );
    assert_eq!(
        loaded[0].moving_block.unwrap().biome_id,
        Some(0),
        "MovingBlockRenderState samples the biome at blockPos"
    );
}

#[test]
fn moving_piston_blocks_go_through_the_block_entity_visibility_gate() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_entity_block_test_chunk(&mut world);
    let base = BlockPos { x: 1, y: 5, z: 5 };
    let arm = BlockPos { x: 2, y: 5, z: 5 };
    let piston = bbb_client_world::vanilla_block_state_id(
        "minecraft:piston",
        &BTreeMap::from([
            ("extended".to_string(), "false".to_string()),
            ("facing".to_string(), "east".to_string()),
        ]),
    )
    .expect("east retracted piston state");
    let stone = bbb_client_world::vanilla_block_state_id("minecraft:stone", &BTreeMap::new())
        .expect("stone state");
    set_entity_block_test_block(&mut world, base, piston);
    set_entity_block_test_block(&mut world, arm, stone);
    assert!(world.apply_block_event(BlockEvent {
        pos: ProtocolBlockPos {
            x: base.x,
            y: base.y,
            z: base.z,
        },
        b0: 0,
        b1: 0,
        block_id: bbb_protocol::block_types::VANILLA_BLOCK_PISTON_ID,
    }));

    // Without a gate every moving block is submitted.
    assert_eq!(
        entity_block_attachments(&[], &world, None, 0.0, MovingPistonVisibility::default()).len(),
        2
    );

    // `PistonHeadRenderer` is an ordinary renderer, so an uncompiled section
    // drops both submissions even though they are metres from the camera.
    let terrain_upload = crate::terrain_upload::TerrainUploadState::default();
    let visibility = crate::block_entity_visibility::BlockEntityVisibility::new(
        Some(bbb_client_render::CameraPose {
            position: [1.5, 5.5, 5.5],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }),
        [1_000, 1_000],
        70.0,
        8,
    );
    assert!(entity_block_attachments(
        &[],
        &world,
        None,
        0.0,
        MovingPistonVisibility::new(visibility, &terrain_upload),
    )
    .is_empty());
}

#[test]
fn entity_block_attachments_submit_interpolated_moving_piston_blocks() {
    let mut world = WorldStore::with_dimension(WorldDimension {
        min_y: 0,
        height: 16,
    });
    insert_entity_block_test_chunk(&mut world);
    let base = BlockPos { x: 1, y: 5, z: 5 };
    let arm = BlockPos { x: 2, y: 5, z: 5 };
    let piston = bbb_client_world::vanilla_block_state_id(
        "minecraft:piston",
        &BTreeMap::from([
            ("extended".to_string(), "false".to_string()),
            ("facing".to_string(), "east".to_string()),
        ]),
    )
    .expect("east retracted piston state");
    let stone = bbb_client_world::vanilla_block_state_id("minecraft:stone", &BTreeMap::new())
        .expect("stone state");
    set_entity_block_test_block(&mut world, base, piston);
    set_entity_block_test_block(&mut world, arm, stone);

    assert!(world.apply_block_event(BlockEvent {
        pos: ProtocolBlockPos {
            x: base.x,
            y: base.y,
            z: base.z,
        },
        b0: 0,
        b1: 0,
        block_id: bbb_protocol::block_types::VANILLA_BLOCK_PISTON_ID,
    }));

    let initial =
        entity_block_attachments(&[], &world, None, 0.0, MovingPistonVisibility::default());
    assert_eq!(initial.len(), 2);
    let initial_head = initial
        .iter()
        .find(|attachment| attachment.block_id == "minecraft:piston_head")
        .expect("moving piston head attachment");
    let initial_stone = initial
        .iter()
        .find(|attachment| attachment.block_id == "minecraft:stone")
        .expect("moved stone attachment");
    assert_eq!(
        initial_head.transform.transform_point3(Vec3::ZERO),
        Vec3::new(1.0, 5.0, 5.0)
    );
    assert_eq!(
        initial_stone.transform.transform_point3(Vec3::ZERO),
        Vec3::new(2.0, 5.0, 5.0)
    );
    assert_eq!(
        initial_head.moving_block,
        Some(MovingBlockRenderContext {
            random_seed_pos: base,
            block_pos: base,
            biome_id: Some(0),
            cardinal_lighting: TerrainCardinalLighting::Default,
        })
    );
    assert_eq!(
        initial_stone.moving_block,
        Some(MovingBlockRenderContext {
            random_seed_pos: arm,
            block_pos: arm,
            biome_id: Some(0),
            cardinal_lighting: TerrainCardinalLighting::Default,
        })
    );

    world.advance_moving_piston_ticks(1);
    let interpolated =
        entity_block_attachments(&[], &world, None, 0.5, MovingPistonVisibility::default());
    assert_eq!(interpolated.len(), 2);
    let interpolated_head = interpolated
        .iter()
        .find(|attachment| attachment.block_id == "minecraft:piston_head")
        .expect("interpolated piston head attachment");
    let interpolated_stone = interpolated
        .iter()
        .find(|attachment| attachment.block_id == "minecraft:stone")
        .expect("interpolated moved stone attachment");
    assert_eq!(
        interpolated_head.transform.transform_point3(Vec3::ZERO),
        Vec3::new(1.25, 5.0, 5.0)
    );
    assert_eq!(
        interpolated_stone.transform.transform_point3(Vec3::ZERO),
        Vec3::new(2.25, 5.0, 5.0)
    );
}

#[test]
fn copper_golem_antenna_attachment_uses_block_state_component_properties() {
    let copper_golem = EntityModelInstance::new(
        28,
        bbb_client_render::EntityModelKind::CopperGolem {
            weathering: bbb_client_render::CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let stack = ItemStackSummary {
        item_id: Some(901),
        count: 1,
        component_patch: DataComponentPatchSummary {
            block_state_properties: BTreeMap::from([
                ("facing".into(), "south".into()),
                ("powered".into(), "true".into()),
            ])
            .into(),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };

    let attachment = copper_golem_antenna_block_attachment_from_stack(
        &copper_golem,
        &stack,
        "minecraft:oak_button",
    )
    .unwrap();

    assert_eq!(attachment.block_id.as_ref(), "minecraft:oak_button");
    assert_eq!(
        attachment.properties.as_ref(),
        &BTreeMap::from([
            ("facing".to_string(), "south".to_string()),
            ("powered".to_string(), "true".to_string()),
        ])
    );
    assert!(attachment
        .transform
        .transform_point3(Vec3::splat(0.5))
        .is_finite());
    assert_eq!(attachment.outline_color, 0);
    assert!(!attachment.outline_only);
    let glowing = copper_golem.with_outline_color(0xff55_aa11);
    let glowing_attachment =
        copper_golem_antenna_block_attachment_from_stack(&glowing, &stack, "minecraft:oak_button")
            .unwrap();
    assert_eq!(glowing_attachment.outline_color, 0xff55_aa11);
    assert!(
        !glowing_attachment.outline_only,
        "BlockDecorationLayer always uses ordinary BlockModelRenderState.submit"
    );
    assert!(copper_golem_antenna_block_attachment_from_stack(
        &EntityModelInstance::snow_golem(121, [0.0, 64.0, 0.0], 0.0),
        &stack,
        "minecraft:oak_button",
    )
    .is_none());
}

#[test]
fn block_ground_transform_seats_a_unit_block_just_above_the_entity_origin() {
    // The 0.25-scaled block centered then lifted +3/16 sits at y in [0.0625, 0.3125], plus the bob.
    let ground_matrix = display_matrix(&BLOCK_GROUND_FALLBACK, false);
    let (min_y, _) = ground_model_bounds(&unit_block_quads(), ground_matrix);
    let min_offset_y = -min_y + 1.0 / 16.0;
    // A full unit block already rests on the ground under the default transform (no extra lift).
    assert!(min_offset_y.abs() < 1e-4, "block needs no lift");
    let phase = 0.75;
    let transform = base_transform([0.0, 64.0, 0.0], 0.0, phase, min_offset_y) * ground_matrix;
    let bob = phase.sin() * 0.1 + 0.1;
    let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.0));
    let top = transform.transform_point3(Vec3::new(1.0, 1.0, 1.0));
    assert!((bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4, "bottom y");
    assert!((top.y - (64.0 + bob + 0.3125)).abs() < 1e-4, "top y");
}

#[test]
fn flat_ground_transform_lifts_the_slab_to_rest_on_the_ground() {
    let ground_matrix = display_matrix(&GENERATED_GROUND_FALLBACK, false);
    let (min_y, _) = ground_model_bounds(&generated_slab_quads(), ground_matrix);
    let min_offset_y = -min_y + 1.0 / 16.0;
    // The 0.5-scaled slab's bottom sits at -0.125, so vanilla lifts it 0.1875 to rest on the ground.
    assert!((min_offset_y - 0.1875).abs() < 1e-4, "flat lift");
    let phase = 0.75;
    let transform = base_transform([0.0, 64.0, 0.0], 0.0, phase, min_offset_y) * ground_matrix;
    let bob = phase.sin() * 0.1 + 0.1;
    let bottom = transform.transform_point3(Vec3::new(0.0, 0.0, 0.5));
    assert!(
        (bottom.y - (64.0 + bob + 0.0625)).abs() < 1e-4,
        "flat bottom y"
    );
}

#[test]
fn custom_ground_transform_reseats_from_its_own_bounds() {
    // A ground transform scaling the block to 0.5 (vs the default 0.25) drops its bottom to -0.25,
    // so vanilla's `-minY + 1/16` lift grows to 0.3125 to keep it resting on the ground — proving the
    // seating is derived per-model from the actual transform, not a hardcoded constant.
    let custom = BlockModelDisplayTransform {
        rotation: [0.0, 0.0, 0.0],
        translation: [0.0, 0.0, 0.0],
        scale: [0.5, 0.5, 0.5],
    };
    let (min_y, _) = ground_model_bounds(&unit_block_quads(), display_matrix(&custom, false));
    assert!(
        (min_y + 0.25).abs() < 1e-4,
        "0.5-scaled block bottom at -0.25"
    );
    let min_offset_y = -min_y + 1.0 / 16.0;
    assert!(
        (min_offset_y - 0.3125).abs() < 1e-4,
        "lift compensates the lower bottom"
    );
}

#[test]
fn rendered_amount_matches_vanilla_thresholds() {
    assert_eq!(rendered_amount(0), 1);
    assert_eq!(rendered_amount(1), 1);
    assert_eq!(rendered_amount(2), 2);
    assert_eq!(rendered_amount(16), 2);
    assert_eq!(rendered_amount(17), 3);
    assert_eq!(rendered_amount(32), 3);
    assert_eq!(rendered_amount(33), 4);
    assert_eq!(rendered_amount(48), 4);
    assert_eq!(rendered_amount(49), 5);
    assert_eq!(rendered_amount(64), 5);
}

#[test]
fn ground_model_bounds_classify_block_vs_flat_depth() {
    // A cube face spanning Z 0..16, scaled 0.25, is 0.25 deep → scatter branch.
    let (_, block_depth) = ground_model_bounds(
        &unit_block_quads(),
        display_matrix(&BLOCK_GROUND_FALLBACK, false),
    );
    assert!((block_depth - 0.25).abs() < 1e-6);
    assert!(block_depth > FLAT_ITEM_DEPTH_THRESHOLD);
    // A generated slab spans Z 7.5..8.5 (depth 1), scaled 0.5 → 0.03125 deep → stack branch.
    let (_, flat_depth) = ground_model_bounds(
        &generated_slab_quads(),
        display_matrix(&GENERATED_GROUND_FALLBACK, false),
    );
    assert!((flat_depth - 0.031_25).abs() < 1e-6);
    assert!(flat_depth <= FLAT_ITEM_DEPTH_THRESHOLD);
}

#[test]
fn custom_head_skull_items_are_reserved_for_the_skull_renderer() {
    for resource_id in [
        "minecraft:skeleton_skull",
        "minecraft:wither_skeleton_skull",
        "minecraft:player_head",
        "minecraft:zombie_head",
        "minecraft:creeper_head",
        "minecraft:dragon_head",
        "minecraft:piglin_head",
    ] {
        assert!(
            bbb_client_world::block_name_is_floor_skull(resource_id),
            "{resource_id} should not use the generic CustomHeadLayer item path"
        );
    }

    assert!(!bbb_client_world::block_name_is_floor_skull(
        "minecraft:carved_pumpkin"
    ));
    assert!(!bbb_client_world::block_name_is_floor_skull(
        "minecraft:white_banner"
    ));
}

#[test]
fn armor_stand_invisible_marker_keeps_held_and_custom_head_item_models() {
    // Vanilla `LivingEntityRenderer.submit` still runs armor-stand `ItemInHandLayer` and the
    // generic item branch of `CustomHeadLayer` when the marker base body has no render type.
    let root = unique_item_model_temp_dir("armor-stand-invisible-held-items");
    write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 601;
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 1)));

    let visible = EntityModelInstance::armor_stand_with_marker(
        ENTITY_ID,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        true,
        true,
        true,
        bbb_client_render::DEFAULT_ARMOR_STAND_MODEL_POSE,
    );
    let hidden_glowing = visible.with_invisible(true).with_outline_color(0xff55_aa11);
    let terrain_textures = TerrainTextureState::default();

    let visible_models =
        held_item_models(&[visible], &world, Some(&item_runtime), &terrain_textures);
    let hidden_models = held_item_models(
        &[hidden_glowing],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    assert!(visible_models.block_meshes.is_empty());
    assert!(hidden_models.block_meshes.is_empty());
    assert_eq!(visible_models.flat_meshes.len(), 2);
    assert_eq!(hidden_models.flat_meshes.len(), 2);
    assert!(visible_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));
    assert!(hidden_models
        .flat_meshes
        .iter()
        .all(|mesh| !mesh.is_empty()));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_invisible_keeps_held_and_custom_head_item_models() {
    // Vanilla `ItemInHandLayer` and the generic item branch of `CustomHeadLayer` have no
    // `state.isInvisible` gate, so ordinary humanoid mobs keep both item-model attachments.
    let root = unique_item_model_temp_dir("zombie-invisible-held-items");
    write_flat_item_runtime_fixture(&root, &["hand_item", "head_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let mut world = WorldStore::new();
    const ENTITY_ID: i32 = 602;
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::Head, 1)));

    let visible = EntityModelInstance::zombie(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let hidden = visible.with_invisible(true);
    let hidden_glowing = visible.with_invisible(true).with_outline_color(0xff55_aa11);
    let terrain_textures = TerrainTextureState::default();

    let visible_models =
        held_item_models(&[visible], &world, Some(&item_runtime), &terrain_textures);
    let hidden_models = held_item_models(&[hidden], &world, Some(&item_runtime), &terrain_textures);
    let glowing_models = held_item_models(
        &[hidden_glowing],
        &world,
        Some(&item_runtime),
        &terrain_textures,
    );

    for models in [&visible_models, &hidden_models, &glowing_models] {
        assert_eq!(models.flat_meshes.len(), 2);
        assert!(models.flat_meshes.iter().all(|mesh| !mesh.is_empty()));
    }

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn humanoid_item_in_hand_layer_uses_main_arm_for_hand_slot_mapping() {
    // Vanilla `ItemInHandLayer.submit` renders RIGHT then LEFT, but each arm resolves its stack via
    // `LivingEntity.getItemHeldByArm(arm)`. A left-main-arm player therefore renders the main-hand
    // stack on the left arm and the off-hand stack on the right arm.
    let root = unique_item_model_temp_dir("humanoid-main-arm-held-items");
    write_flat_item_runtime_fixture(&root, &["main_item", "off_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    const ENTITY_ID: i32 = 606;
    let terrain_textures = TerrainTextureState::default();

    let mut main_world = WorldStore::new();
    main_world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(main_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let right_main = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let left_main = right_main.with(|s| s.main_arm_left = true);
    let right_main_models = held_item_models(
        &[right_main],
        &main_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let left_main_models = held_item_models(
        &[left_main],
        &main_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    assert_eq!(right_main_models.flat_meshes.len(), 1);
    assert_eq!(left_main_models.flat_meshes.len(), 1);
    assert_ne!(
        right_main_models.flat_meshes[0], left_main_models.flat_meshes[0],
        "main-hand item moves from the right arm to the left arm when mainArm is LEFT"
    );

    let mut off_world = WorldStore::new();
    off_world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(off_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::OffHand, 1)));
    let right_off_models = held_item_models(
        &[right_main],
        &off_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    let left_off_models = held_item_models(
        &[left_main],
        &off_world,
        Some(&item_runtime),
        &terrain_textures,
    );
    assert_eq!(right_off_models.flat_meshes.len(), 1);
    assert_eq!(left_off_models.flat_meshes.len(), 1);
    assert_ne!(
        right_off_models.flat_meshes[0], left_off_models.flat_meshes[0],
        "off-hand item moves from the left arm to the right arm when mainArm is LEFT"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn player_scoping_spyglass_renders_held_to_eye_instead_of_in_hand() {
    // Vanilla `PlayerItemInHandLayer.submitArmWithItem`: an avatar using a spyglass in the rendered
    // hand outside a melee swing (`attackTime < 1e-5`) replaces the in-hand submit with
    // `renderItemHeldToEye` — the stack resolved at `ItemDisplayContext.HEAD` on the head bone with
    // `head.xRot` clamped to `[-π/6, π/2]`. Non-avatar humanoids keep the generic `ItemInHandLayer`
    // in-hand render.
    let root = unique_item_model_temp_dir("player-spyglass-held-to-eye");
    write_flat_item_runtime_fixture(&root, &["spyglass"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    const ENTITY_ID: i32 = 611;
    let terrain_textures = TerrainTextureState::default();
    let mut world = WorldStore::new();
    world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));

    let bake = |instance: EntityModelInstance| {
        let models = held_item_models(&[instance], &world, Some(&item_runtime), &terrain_textures);
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes.into_iter().next().unwrap()
    };

    let idle = EntityModelInstance::player(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let scoping = idle.with(|s| s.is_using_item = true);
    let idle_mesh = bake(idle);
    let scoping_mesh = bake(scoping);
    assert_ne!(
        idle_mesh, scoping_mesh,
        "scoping moves the spyglass off the hand attach onto the head bone"
    );

    // The eye attach clamps `head.xRot` to `-π/6` looking up: `-80°` and the exact `-30°` boundary
    // land on the same clamped head pose, while an in-range pitch still tracks the head.
    let clamped_mesh = bake(scoping.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -80.0)));
    let boundary_mesh = bake(scoping.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -30.0)));
    let tracking_mesh = bake(scoping.with(|s| (s.head_yaw, s.head_pitch) = (0.0, -20.0)));
    assert_eq!(
        clamped_mesh, boundary_mesh,
        "pitch beyond the clamp renders exactly at the -π/6 clamp boundary"
    );
    assert_ne!(
        tracking_mesh, boundary_mesh,
        "pitch inside the clamp range tracks the posed head"
    );

    // `attackTime < 1e-5` gates the branch: a mid-swing scoping player renders in hand again. The
    // eye transform itself is attack-invariant (root · head only), so any difference proves the
    // branch switched.
    let swinging_mesh = bake(scoping.with(|s| s.attack_anim = 0.4));
    assert_ne!(
        swinging_mesh, scoping_mesh,
        "a melee swing suppresses the held-to-eye render"
    );

    // Using the off hand while the spyglass sits in the main hand renders the main arm normally.
    let wrong_hand_mesh = bake(scoping.with(|s| s.use_item_off_hand = true));
    assert_eq!(
        wrong_hand_mesh, idle_mesh,
        "the held-to-eye branch requires the rendered hand to be the use hand"
    );

    // A non-avatar humanoid using a spyglass keeps the generic in-hand render.
    let mut zombie_world = WorldStore::new();
    zombie_world.apply_add_entity(protocol_add_entity(
        ENTITY_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
    ));
    assert!(zombie_world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    let zombie_bake = |instance: EntityModelInstance| {
        let models = held_item_models(
            &[instance],
            &zombie_world,
            Some(&item_runtime),
            &terrain_textures,
        );
        assert_eq!(models.flat_meshes.len(), 1);
        models.flat_meshes.into_iter().next().unwrap()
    };
    let zombie = EntityModelInstance::zombie(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let zombie_idle_mesh = zombie_bake(zombie);
    let zombie_scoping_mesh = zombie_bake(zombie.with(|s| s.is_using_item = true));
    assert_eq!(
        zombie_idle_mesh, zombie_scoping_mesh,
        "only avatar models install PlayerItemInHandLayer's held-to-eye branch"
    );

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn living_held_item_team_tint_uses_owner_scoreboard_color() {
    use bbb_protocol::packets::{
        ChatFormatting, PlayerTeamMethod, PlayerTeamParameters, SetPlayerTeam, TeamCollisionRule,
        TeamVisibility,
    };

    let root = unique_item_model_temp_dir("held-team-tint");
    write_team_tint_item_runtime_fixture(&root);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    const ENTITY_ID: i32 = 607;
    let mut world = WorldStore::new();
    let entity = protocol_add_entity(ENTITY_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let entity_uuid = entity.uuid;
    world.apply_add_entity(entity);
    assert!(world.apply_set_equipment(equipment(ENTITY_ID, EquipmentSlot::MainHand, 0)));
    assert!(world.apply_set_player_team(SetPlayerTeam {
        name: "green".into(),
        method: PlayerTeamMethod::Add.into(),
        parameters: Some(PlayerTeamParameters {
            display_name: bbb_protocol::RichTextComponent::plain("Green"),
            options: 0,
            nametag_visibility: TeamVisibility::Always,
            collision_rule: TeamCollisionRule::Always,
            color: ChatFormatting::Green,
            player_prefix: bbb_protocol::RichTextComponent::plain(""),
            player_suffix: bbb_protocol::RichTextComponent::plain(""),
        }),
        players: vec![entity_uuid.to_string().into()],
    }));

    let instance = EntityModelInstance::zombie(ENTITY_ID, [0.0, 64.0, 0.0], 0.0, false);
    let models = held_item_models(
        &[instance],
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
    );
    assert!(!models.residency.instances.is_empty());
    assert!(models
        .residency
        .instances
        .iter()
        .all(|instance| instance.tint == [0x55 as f32 / 255.0, 1.0, 0x55 as f32 / 255.0, 1.0]));

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_bake_ordinary_local_hand_stack() {
    let root = unique_item_model_temp_dir("first-person-basic-item");
    write_flat_item_runtime_fixture(&root, &["hand_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(CameraPose {
            position: [4.0, 64.0, -2.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: CameraPose::STANDING_EYE_HEIGHT,
        }),
        1.0,
    );
    assert_eq!(models.flat_meshes.len(), 1);
    assert_eq!(models.residency.assets.len(), 1);
    assert_eq!(models.residency.instances.len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_apply_local_player_whack_swing() {
    let root = unique_item_model_temp_dir("first-person-whack-swing");
    write_flat_item_runtime_fixture(&root, &["hand_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let mut world = world_with_level_dimension("minecraft:overworld");
    world.apply_add_entity(protocol_add_entity(1, VANILLA_ENTITY_TYPE_PLAYER_ID));
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });

    let still = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(still.flat_meshes.len(), 1);

    assert!(world.apply_entity_animation(EntityAnimation { id: 1, action: 0 }));
    world.advance_entity_client_animations(2);
    let swinging = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(swinging.flat_meshes.len(), 1);
    assert_ne!(
        still.flat_meshes[0], swinging.flat_meshes[0],
        "ordinary first-person WHACK items should receive vanilla swingArm transform"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_render_the_item_activation_totem_pop() {
    let root = unique_item_model_temp_dir("first-person-item-activation");
    write_flat_item_runtime_fixture(&root, &["hand_item"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });
    let terrain = TerrainTextureState::default();
    let mut world = world_with_level_dimension("minecraft:overworld");
    world.apply_add_entity(protocol_add_entity(
        1,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    ));
    let mut patch = DataComponentPatchSummary::default();
    patch
        .added_type_ids
        .push(bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DEATH_PROTECTION_TYPE_ID);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
            count: 1,
            component_patch: patch.into(),
        },
    });

    let before = first_person_item_models(&world, Some(&item_runtime), &terrain, camera, 0.0);
    assert_eq!(
        before.flat_meshes.len(),
        1,
        "only the held stack renders before the totem event"
    );

    let mut random = bbb_client_world::LevelEventSoundRandomState::with_seed(0);
    assert!(world
        .apply_play_packet(
            bbb_protocol::packets::PlayClientbound::EntityEvent(EntityEvent {
                entity_id: 1,
                event_id: 35,
            }),
            &mut random,
            &mut bbb_client_world::NoPlayApplyEffects,
        )
        .is_none());
    let with_totem = first_person_item_models(&world, Some(&item_runtime), &terrain, camera, 0.0);
    assert_eq!(
        with_totem.flat_meshes.len(),
        2,
        "the activated stack pops beside the held stack"
    );

    // Vanilla gates `renderItemActivationAnimation` only on hideGui: a
    // sleeping camera player suppresses the hands but keeps the pop.
    let accessors = bbb_client_world::vanilla_entity_metadata_entity_accessors()
        .expect("generated base Entity metadata accessors must resolve");
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 1,
        values: vec![EntityDataValue {
            data_id: accessors.pose_data_id,
            serializer_id:
                bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_POSE_ID,
            value: EntityDataValueKind::Pose(
                bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID
            ),
        }],
    }));
    let sleeping = first_person_item_models(&world, Some(&item_runtime), &terrain, camera, 0.0);
    assert_eq!(
        sleeping.flat_meshes.len(),
        1,
        "the sleeping gate hides the hands, not the item activation"
    );

    // Third-person F5 (first_person_camera_pose = None) still renders totem activation
    // via activation_camera_pose while hands stay hidden, and held items inherit player light.
    let mut map_cache = MapTextureExtractionCache::default();
    map_cache.begin_frame();
    let hand_presentation = FirstPersonHandPresentation::fully_raised_for_test(&world);
    let registry_keys = ItemRegistryKeySet::from_world(&world);
    let mut dark_player = bbb_client_render::EntityModelInstance::player_with_parts(
        1,
        [0.0, 64.0, 0.0],
        0.0,
        false,
        bbb_client_render::PlayerModelPartVisibility::from_vanilla_mask(0),
    );
    dark_player.render_state.light_coords = (3 << 4) | (5 << 20);
    let third_person_totem = crate::item_models::test_meshes::ResidentMeshView::new(
        crate::item_models::first_person_item_models_for_frame(
            &world,
            &hand_presentation,
            Some(&item_runtime),
            &terrain,
            None,
            &registry_keys,
            &mut map_cache,
            None,
            camera,
            Some(&dark_player),
            0.0,
            1.0,
        ),
        |models| &models.residency,
    );
    assert_eq!(third_person_totem.flat_meshes.len(), 1);
    assert_eq!(
        third_person_totem.flat_meshes[0].vertices[0].light,
        [1.0, 1.0],
        "totem activation remains full-bright in third person"
    );

    // `ScreenEffectRenderer.tick` nulls the item at zero and the pop stops.
    world.advance_item_activation_ticks(40);
    let expired = first_person_item_models(&world, Some(&item_runtime), &terrain, camera, 0.0);
    assert!(expired.flat_meshes.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn item_activation_view_transform_matches_the_vanilla_pose_stack() {
    // tick 10 of 40, partial 0.5, offX 1, offY -1, aspect 2: probe points
    // computed independently from the vanilla formula
    // (translate(offX*0.3*aspect*|sin(2πs)|, offY*0.3*|sin(2πs)|, -10+9sin(πs))
    // * scale(0.8) * rotY(900|sin(πs)|°) * rotX(6cos(8*scale)°)
    // * rotZ(6cos(8*scale)°)).
    let transform = item_activation_view_transform(30, 1.0, -1.0, 0.5, 2.0);
    let origin = transform.transform_point3(Vec3::ZERO);
    let forward = transform.transform_point3(Vec3::Z);
    assert!(
        origin.abs_diff_eq(Vec3::new(0.168_676, -0.084_338, -1.091_203), 1.0e-4),
        "{origin:?}"
    );
    assert!(
        forward.abs_diff_eq(Vec3::new(0.295_306, -0.042_064, -1.879_986), 1.0e-4),
        "{forward:?}"
    );
}

#[test]
fn first_person_item_models_skip_unsupported_using_and_missing_map_data() {
    let root = unique_item_model_temp_dir("first-person-special-skip");
    write_flat_item_runtime_fixture(&root, &["hand_item", "filled_map"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let hand_item = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:hand_item"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    let mut filled_map = ItemStackSummary {
        item_id: item_runtime.item_protocol_id("minecraft:filled_map"),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    };
    filled_map.component_patch.map_id = Some(7);
    let camera = Some(CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 0.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    });

    let mut using_world = WorldStore::new();
    using_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: hand_item,
    });
    using_world.set_local_using_item(true);
    assert!(
        !first_person_item_models(
            &using_world,
            Some(&item_runtime),
            &TerrainTextureState::default(),
            camera,
            1.0,
        )
        .flat_meshes
        .is_empty(),
        "unsupported use animation falls back to idle hand pose instead of hiding hands"
    );

    let mut special_world = WorldStore::new();
    special_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: filled_map,
    });
    assert!(first_person_item_models(
        &special_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    )
    .flat_meshes
    .is_empty());
    assert!(first_person_item_models(
        &special_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    )
    .map_surfaces
    .is_empty());
    let missing_map = first_person_item_models(
        &special_world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        camera,
        1.0,
    );
    assert_eq!(missing_map.map_background_surfaces.len(), 1);
    assert_eq!(
        missing_map.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Plain
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_render_two_handed_filled_map_surface() {
    let root = unique_item_model_temp_dir("first-person-map-two-handed");
    write_flat_item_runtime_fixture(&root, &["filled_map"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let packed_grass_high = (1 << 2) | 2;
    let map = first_person_test_map_stack(&item_runtime, 7);
    let pose = CameraPose {
        position: [2.0, 65.0, -4.0],
        y_rot: 35.0,
        x_rot: -15.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item: map });
    assert!(world.apply_map_item_data(MapItemData {
        map_id: 7,
        scale: 0,
        locked: false,
        decorations: Some(Vec::new()),
        color_patch: Some(MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 1,
            height: 1,
            colors: vec![packed_grass_high],
        }),
    }));

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(pose),
        1.0,
    );
    assert_eq!(models.map_background_surfaces.len(), 1);
    assert_eq!(
        models.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(
        models.map_background_surfaces[0].submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            0.0,
            0.0,
        )
    );
    assert_eq!(models.map_textures.len(), 1);
    assert_eq!(models.map_textures[0].map_id, 7);
    assert_eq!(
        &models.map_textures[0].rgba[0..4],
        &map_color_rgba8(packed_grass_high)
    );
    assert_eq!(models.map_surfaces.len(), 1);
    let surface = &models.map_surfaces[0];
    assert!(!surface.is_empty());
    assert_eq!(surface.vertex_count(), 4);
    assert_eq!(surface.index_count(), 6);
    assert_eq!(surface.submission.map_id, 7);
    assert_eq!(surface.submission.texture.vanilla_path(), "minecraft:map/7");
    assert_eq!(surface.submission.light, ITEM_MODEL_FULL_BRIGHT_LIGHT);
    assert_eq!(
        surface.submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            0.0,
            0.0,
        )
    );
    let mut lowered_presentation = FirstPersonHandPresentation::fully_raised_for_test(&world);
    lowered_presentation.item_used(InteractionHand::MainHand);
    let mut map_texture_cache = MapTextureExtractionCache::default();
    map_texture_cache.begin_frame();
    let registry_keys = ItemRegistryKeySet::from_world(&world);
    let lowered = first_person_item_models_with_map_cache(
        &world,
        &lowered_presentation,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        None,
        &registry_keys,
        &mut map_texture_cache,
        Some(pose),
        1.0,
        1.0,
    );
    assert_eq!(
        lowered.map_surfaces[0].submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            1.0,
            0.0,
        ),
        "first-person item extraction consumes the published inverse hand height",
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_person_item_models_render_one_handed_offhand_filled_map_surface() {
    let root = unique_item_model_temp_dir("first-person-map-one-handed");
    write_flat_item_runtime_fixture(&root, &["filled_map"]);
    let item_runtime =
        NativeItemRuntime::load(&bbb_client_assets::PackRoots::from_root(&root).unwrap()).unwrap();
    let map = first_person_test_map_stack(&item_runtime, 8);
    let pose = CameraPose {
        position: [0.0, 64.0, 0.0],
        y_rot: 0.0,
        x_rot: 20.0,
        eye_height: CameraPose::STANDING_EYE_HEIGHT,
    };
    let mut world = WorldStore::new();
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 40,
        item: map,
    });
    assert!(world.apply_map_item_data(MapItemData {
        map_id: 8,
        scale: 0,
        locked: false,
        decorations: Some(Vec::new()),
        color_patch: Some(MapColorPatch {
            start_x: 0,
            start_y: 0,
            width: 1,
            height: 1,
            colors: vec![(2 << 2) | 1],
        }),
    }));

    let models = first_person_item_models(
        &world,
        Some(&item_runtime),
        &TerrainTextureState::default(),
        Some(pose),
        1.0,
    );
    assert_eq!(models.map_background_surfaces.len(), 1);
    assert_eq!(
        models.map_background_surfaces[0].submission.kind,
        FirstPersonMapBackgroundKind::Checkerboard
    );
    assert_eq!(models.map_surfaces.len(), 1);
    let expected = first_person_one_handed_map_transform(
        first_person_camera_world_transform(pose),
        true,
        0.0,
        0.0,
    );
    assert_eq!(models.map_surfaces[0].submission.transform, expected);
    assert_ne!(
        models.map_surfaces[0].submission.transform,
        first_person_two_handed_map_transform(
            first_person_camera_world_transform(pose),
            pose.x_rot,
            0.0,
            0.0,
        ),
        "offhand maps use vanilla's one-handed map branch"
    );
    std::fs::remove_dir_all(root).unwrap();
}
