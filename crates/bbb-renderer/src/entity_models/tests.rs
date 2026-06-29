use super::*;

mod allay;
mod armadillo;
mod armor;
mod armor_stand;
mod arrow;
mod auto_spin;
mod axolotl;
mod bat;
mod bee;
mod blaze;
mod boat;
mod breeze;
mod camel;
mod chicken;
mod cod;
mod copper_golem;
mod cow;
mod creaking;
mod creeper;
mod custom_head_skull;
mod death;
mod dolphin;
mod end_crystal;
mod ender_dragon;
mod enderman;
mod endermite;
mod equine;
mod evoker_fangs;
mod feline;
mod fox;
mod frog;
mod ghast;
mod giant;
mod goat;
mod golem;
mod guardian;
mod happy_ghast;
mod head_look;
mod hoglin;
mod illager;
mod keyframe;
mod leash_knot;
mod limb_swing;
mod llama;
mod llama_spit;
mod minecart;
mod model;
mod mooshroom;
mod nautilus;
mod no_render;
mod panda;
mod parrot;
mod phantom;
mod pig;
mod piglin;
mod player;
mod polar_bear;
mod pufferfish;
mod rabbit;
mod ravager;
mod salmon;
mod scale;
mod sheep;
mod shulker;
mod shulker_bullet;
mod silverfish;
mod skeleton;
mod sleeping;
mod slime;
mod sniffer;
mod spider;
mod squid;
mod strider;
mod tadpole;
mod trident;
mod tropical_fish;
mod turtle;
mod upside_down;
mod vex;
mod villager;
mod warden;
mod wind_charge;
mod wings;
mod witch;
mod wither;
mod wither_skull;
mod wolf;
mod zombie;

#[test]
fn textured_layer_render_type_names_match_vanilla_render_types() {
    let cases = [
        (
            EntityModelLayerRenderType::EntitySolid,
            "entitySolid",
            EntityModelLayerRenderBucket::Cutout,
            true,
            true,
            false,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            "armorCutoutNoCull",
            EntityModelLayerRenderBucket::Cutout,
            true,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::ArmorTranslucent,
            "armorTranslucent",
            EntityModelLayerRenderBucket::Translucent,
            true,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::EntityCutout,
            "entityCutout",
            EntityModelLayerRenderBucket::Cutout,
            true,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::EntityCutoutCull,
            "entityCutoutCull",
            EntityModelLayerRenderBucket::Cutout,
            true,
            true,
            false,
        ),
        (
            EntityModelLayerRenderType::EntityCutoutZOffset,
            "entityCutoutZOffset",
            EntityModelLayerRenderBucket::Cutout,
            true,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::EntityTranslucent,
            "entityTranslucent",
            EntityModelLayerRenderBucket::Translucent,
            true,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::EntityTranslucentCullItemTarget,
            "entityTranslucentCullItemTarget",
            EntityModelLayerRenderBucket::ItemEntityTranslucent,
            true,
            true,
            true,
        ),
        (
            EntityModelLayerRenderType::Outline,
            "outline",
            EntityModelLayerRenderBucket::OutlineOnly,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::EntityGlint,
            "entityGlint",
            EntityModelLayerRenderBucket::GlintOnly,
            false,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::Eyes,
            "eyes",
            EntityModelLayerRenderBucket::Eyes,
            false,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::BreezeWind,
            "breezeWind",
            EntityModelLayerRenderBucket::Scroll,
            false,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::EnergySwirl,
            "energySwirl",
            EntityModelLayerRenderBucket::AdditiveScroll,
            false,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::EndCrystalBeam,
            "end_crystal_beam",
            EntityModelLayerRenderBucket::Scroll,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::WaterMask,
            "waterMask",
            EntityModelLayerRenderBucket::DepthOnly,
            false,
            false,
            false,
        ),
    ];

    assert_eq!(cases.len(), EntityModelLayerRenderType::ALL.len());
    for (render_type, vanilla_name, bucket, affects_outline, surface_cull, has_blending) in cases {
        assert_eq!(render_type.vanilla_name(), vanilla_name);
        assert_eq!(render_type.mesh_bucket(), bucket);
        assert_eq!(render_type.affects_outline(), affects_outline);
        assert_eq!(render_type.surface_cull(), surface_cull);
        assert_eq!(render_type.outline_cull(), surface_cull);
        assert_eq!(render_type.has_blending(), has_blending);
    }
}

#[test]
fn texture_backed_blended_model_uploads_sort_by_order_then_camera_distance() {
    let near = EntityModelInstance::sheep(701, [2.0, 64.0, 0.0], 0.0, false)
        .with_invisible(true)
        .with_invisible_to_player(false);
    let far = EntityModelInstance::sheep(702, [10.0, 64.0, 0.0], 0.0, false)
        .with_invisible(true)
        .with_invisible_to_player(false);
    let images: Vec<_> = sheep_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = (texture.size[0] * texture.size[1] * 4) as usize;
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();

    let near_only = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[near],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
    );
    let far_only = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[far],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
    );
    assert_eq!(
        near_only.item_entity_translucent_cull.vertices.len(),
        far_only.item_entity_translucent_cull.vertices.len()
    );

    let sorted_from_origin = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[near, far],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
    );
    assert_eq!(sorted_from_origin.submissions.len(), 2);
    assert_eq!(
        sorted_from_origin.submissions[0]
            .transform
            .transform_point3(Vec3::ZERO)
            .x,
        near.position[0]
    );
    let far_vertex_count = far_only.item_entity_translucent_cull.vertices.len();
    assert_eq!(
        &sorted_from_origin.item_entity_translucent_cull.vertices[..far_vertex_count],
        far_only.item_entity_translucent_cull.vertices.as_slice()
    );
    assert_eq!(sorted_from_origin.sorted_item_entity_draws.len(), 2);
    assert_eq!(
        sorted_from_origin.sorted_item_entity_draws[0],
        EntityModelTexturedDrawRange {
            atlas: EntityModelTexturedDrawAtlas::Static,
            surface_cull: true,
            index_start: 0,
            index_count: far_only.item_entity_translucent_cull.indices.len() as u32,
        }
    );
    assert_eq!(
        sorted_from_origin.sorted_item_entity_draws[1],
        EntityModelTexturedDrawRange {
            atlas: EntityModelTexturedDrawAtlas::Static,
            surface_cull: true,
            index_start: far_only.item_entity_translucent_cull.indices.len() as u32,
            index_count: near_only.item_entity_translucent_cull.indices.len() as u32,
        }
    );

    let sorted_from_positive_x = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[near, far],
        &atlas,
        None,
        None,
        Some([20.0, 64.0, 0.0]),
    );
    let near_vertex_count = near_only.item_entity_translucent_cull.vertices.len();
    assert_eq!(
        &sorted_from_positive_x.item_entity_translucent_cull.vertices[..near_vertex_count],
        near_only.item_entity_translucent_cull.vertices.as_slice()
    );
    assert_eq!(
        sorted_from_positive_x.sorted_item_entity_draws[0].index_count,
        near_only.item_entity_translucent_cull.indices.len() as u32
    );
}

#[test]
fn texture_backed_blended_draw_plan_preserves_cross_atlas_distance_order() {
    // Vanilla ModelFeatureRenderer stores every hasBlending() model submit in one list and sorts the
    // list by camera distance before drawing, independent of the texture atlas that backs the submit.
    let static_head = EntityModelInstance::player_with_parts(
        801,
        [2.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Player(
        EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve),
    )));
    let dynamic_skin = EntityDynamicPlayerSkin {
        handle: 8802,
        fallback: EntityDefaultPlayerSkin::WideSteve,
        model: EntityPlayerSkinModel::Wide,
        status: EntityDynamicPlayerSkinStatus::Ready,
    };
    let dynamic_head = EntityModelInstance::player_with_parts(
        802,
        [10.0, 64.0, 0.0],
        0.0,
        false,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    )
    .with_custom_head_skull(Some(EntityCustomHeadSkull::Player(
        EntityPlayerSkin::Dynamic(dynamic_skin),
    )));
    let len = (PLAYER_WIDE_STEVE_TEXTURE_REF.size[0] * PLAYER_WIDE_STEVE_TEXTURE_REF.size[1] * 4)
        as usize;
    let (atlas, _) = build_entity_model_texture_atlas(&[EntityModelTextureImage::new(
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        vec![0x66; len],
    )])
    .unwrap();
    let (dynamic_atlas, _) =
        build_dynamic_player_skin_atlas(&[crate::player_skin::DynamicPlayerSkinImage {
            handle: dynamic_skin.handle,
            rgba: vec![0xdd; 64 * 64 * 4],
        }])
        .unwrap();

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[static_head, dynamic_head],
        &atlas,
        Some(&dynamic_atlas),
        None,
        Some([0.0, 64.0, 0.0]),
    );

    assert_eq!(meshes.sorted_translucent_draws.len(), 2);
    assert_eq!(
        meshes
            .sorted_translucent_draws
            .iter()
            .map(|draw| draw.atlas)
            .collect::<Vec<_>>(),
        vec![
            EntityModelTexturedDrawAtlas::DynamicPlayerSkin,
            EntityModelTexturedDrawAtlas::Static
        ]
    );
    assert!(meshes
        .sorted_translucent_draws
        .iter()
        .all(|draw| !draw.surface_cull));
    assert_eq!(
        meshes.sorted_translucent_draws[0].index_count,
        meshes.dynamic_player_skin_translucent.indices.len() as u32
    );
    assert_eq!(
        meshes.sorted_translucent_draws[1].index_count,
        meshes.translucent.indices.len() as u32
    );
    assert_eq!(meshes.sorted_translucent_draws[0].index_start, 0);
    assert_eq!(meshes.sorted_translucent_draws[1].index_start, 0);
}

#[test]
fn equipment_layer_pass_records_vanilla_model_layer_metadata() {
    let cases = [
        (
            EntityModelLayerKind::PigSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_PIG_SADDLE,
            PIG_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::StriderSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_STRIDER_SADDLE,
            STRIDER_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::CamelSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_CAMEL_HUSK_SADDLE,
            CAMEL_HUSK_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::NautilusBodyArmor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_NAUTILUS_ARMOR,
            NAUTILUS_BODY_IRON_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::NautilusSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_NAUTILUS_SADDLE,
            NAUTILUS_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            2,
        ),
        (
            EntityModelLayerKind::LlamaDecor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_LLAMA_BABY_DECOR,
            LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            1,
            1,
        ),
        (
            EntityModelLayerKind::WolfBodyArmor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_WOLF_ARMOR,
            WOLF_BODY_ARMADILLO_SCUTE_OVERLAY_TEXTURE_REF,
            [
                0x33 as f32 / 255.0,
                0x66 as f32 / 255.0,
                0x99 as f32 / 255.0,
                1.0,
            ],
            2,
            3,
        ),
        (
            EntityModelLayerKind::WolfBodyArmorCrack,
            EntityModelLayerRenderType::ArmorTranslucent,
            MODEL_LAYER_WOLF_ARMOR,
            WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            3,
            4,
        ),
        (
            EntityModelLayerKind::EquineSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_HORSE_SADDLE,
            HORSE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            2,
            3,
        ),
        (
            EntityModelLayerKind::EquineSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_DONKEY_SADDLE,
            DONKEY_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::EquineSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_MULE_SADDLE,
            MULE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::EquineSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_SKELETON_HORSE_SADDLE,
            SKELETON_HORSE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
        (
            EntityModelLayerKind::EquineSaddle,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_ZOMBIE_HORSE_SADDLE,
            ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            2,
        ),
        (
            EntityModelLayerKind::EquineBodyArmor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_HORSE_ARMOR,
            HORSE_BODY_LEATHER_TEXTURE_REF,
            armor_layer_tint(EntityArmorMaterial::Leather, Some(0x003F_6CDA)),
            2,
            2,
        ),
        (
            EntityModelLayerKind::EquineBodyArmor,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MODEL_LAYER_UNDEAD_HORSE_ARMOR,
            HORSE_BODY_IRON_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
            1,
        ),
    ];

    for (kind, render_type, model_layer, texture, tint, order, submit_sequence) in cases {
        let pass = equipment_layer_pass(
            kind,
            render_type,
            model_layer,
            texture,
            tint,
            order,
            submit_sequence,
        );
        assert_eq!(pass.kind, kind);
        assert_eq!(pass.render_type, render_type);
        assert_eq!(
            pass.render_type.vanilla_name(),
            match render_type {
                EntityModelLayerRenderType::ArmorCutoutNoCull => "armorCutoutNoCull",
                EntityModelLayerRenderType::ArmorTranslucent => "armorTranslucent",
                _ => panic!("unexpected equipment render type"),
            }
        );
        assert_eq!(pass.model_layer, model_layer);
        assert_eq!(pass.texture, texture);
        assert_eq!(pass.visibility, EntityModelLayerVisibility::All);
        assert_eq!(pass.tint, tint);
        assert_eq!((pass.order, pass.submit_sequence), (order, submit_sequence));
    }
}

#[test]
fn runtime_colored_mesh_excludes_texture_backed_entities() {
    let chicken = EntityModelInstance::chicken(303, [-2.0, 64.0, 0.0], 0.0, false);
    let sheep = EntityModelInstance::sheep(304, [0.0, 64.0, 0.0], 0.0, false);
    let wolf = EntityModelInstance::wolf(305, [2.0, 64.0, 0.0], 0.0, false);
    let boat = EntityModelInstance::boat(306, [4.0, 64.0, 0.0], 0.0, BoatModelFamily::Oak, true);
    let pig = EntityModelInstance::pig(
        307,
        [6.0, 64.0, 0.0],
        0.0,
        PigModelVariant::Temperate,
        false,
    );
    let cow =
        EntityModelInstance::cow_variant(308, [8.0, 64.0, 0.0], 0.0, CowModelVariant::Warm, false);
    let player = EntityModelInstance::player(309, [10.0, 64.0, 0.0], 0.0, false);
    let creeper = EntityModelInstance::new(310, EntityModelKind::Creeper, [12.0, 64.0, 0.0], 0.0);
    let spider = EntityModelInstance::spider(311, [14.0, 64.0, 0.0], 0.0);
    let cave_spider = EntityModelInstance::cave_spider(312, [16.0, 64.0, 0.0], 0.0);
    let enderman = EntityModelInstance::enderman(313, [18.0, 64.0, 0.0], 0.0);
    let slime = EntityModelInstance::slime(314, [20.0, 64.0, 0.0], 0.0, 1);
    let magma_cube = EntityModelInstance::magma_cube(315, [22.0, 64.0, 0.0], 0.0, 3);
    let goat = EntityModelInstance::goat(316, [24.0, 64.0, 0.0], 0.0, false, true, false);
    let polar_bear = EntityModelInstance::polar_bear(317, [26.0, 64.0, 0.0], 0.0, false);
    let hoglin = EntityModelInstance::hoglin(
        318,
        [28.0, 64.0, 0.0],
        0.0,
        HoglinModelFamily::Hoglin,
        false,
    );
    let zoglin_baby =
        EntityModelInstance::hoglin(319, [30.0, 64.0, 0.0], 0.0, HoglinModelFamily::Zoglin, true);
    let piglin = EntityModelInstance::piglin(
        320,
        [32.0, 64.0, 0.0],
        0.0,
        PiglinModelFamily::Piglin,
        false,
    );
    let ravager = EntityModelInstance::ravager(321, [34.0, 64.0, 0.0], 0.0);
    let villager = EntityModelInstance::villager(322, [36.0, 64.0, 0.0], 0.0, false);
    let wandering_trader = EntityModelInstance::wandering_trader(323, [38.0, 64.0, 0.0], 0.0);
    let iron_golem = EntityModelInstance::iron_golem(324, [40.0, 64.0, 0.0], 0.0);
    let snow_golem = EntityModelInstance::snow_golem(325, [42.0, 64.0, 0.0], 0.0);
    let witch = EntityModelInstance::witch(326, [44.0, 64.0, 0.0], 0.0);
    let copper_golem = EntityModelInstance::new(
        327,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [46.0, 64.0, 0.0],
        0.0,
    );
    let husk = EntityModelInstance::zombie_variant(
        328,
        [48.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Husk,
        false,
    );
    let drowned = EntityModelInstance::zombie_variant(
        329,
        [50.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::Drowned,
        false,
    );
    let zombie_villager = EntityModelInstance::zombie_variant(
        330,
        [52.0, 64.0, 0.0],
        0.0,
        ZombieVariantModelFamily::ZombieVillager,
        false,
    );
    let colored = entity_model_colored_runtime_mesh(&[
        chicken,
        sheep,
        wolf,
        boat,
        pig,
        cow,
        player,
        creeper,
        spider,
        cave_spider,
        enderman,
        slime,
        magma_cube,
        goat,
        polar_bear,
        hoglin,
        zoglin_baby,
        piglin,
        ravager,
        villager,
        wandering_trader,
        iron_golem,
        snow_golem,
        witch,
        copper_golem,
        husk,
        drowned,
        zombie_villager,
    ]);
    assert!(colored.vertices.is_empty());
    assert!(colored.indices.is_empty());
    let legacy_chicken_geometry_guard = entity_model_mesh(&[chicken]);
    assert!(!legacy_chicken_geometry_guard.vertices.is_empty());
    let legacy_geometry_guard = entity_model_mesh(&[sheep]);
    assert!(!legacy_geometry_guard.vertices.is_empty());
    let legacy_wolf_geometry_guard = entity_model_mesh(&[wolf]);
    assert!(!legacy_wolf_geometry_guard.vertices.is_empty());
    let legacy_boat_geometry_guard = entity_model_mesh(&[boat]);
    assert!(!legacy_boat_geometry_guard.vertices.is_empty());
    let legacy_pig_geometry_guard = entity_model_mesh(&[pig]);
    assert!(!legacy_pig_geometry_guard.vertices.is_empty());
    let legacy_cow_geometry_guard = entity_model_mesh(&[cow]);
    assert!(!legacy_cow_geometry_guard.vertices.is_empty());
    let legacy_player_geometry_guard = entity_model_mesh(&[player]);
    assert!(!legacy_player_geometry_guard.vertices.is_empty());
    let legacy_creeper_geometry_guard = entity_model_mesh(&[creeper]);
    assert!(!legacy_creeper_geometry_guard.vertices.is_empty());
    let legacy_spider_geometry_guard = entity_model_mesh(&[spider]);
    assert!(!legacy_spider_geometry_guard.vertices.is_empty());
    let legacy_cave_spider_geometry_guard = entity_model_mesh(&[cave_spider]);
    assert!(!legacy_cave_spider_geometry_guard.vertices.is_empty());
    let legacy_enderman_geometry_guard = entity_model_mesh(&[enderman]);
    assert!(!legacy_enderman_geometry_guard.vertices.is_empty());
    let legacy_slime_geometry_guard = entity_model_mesh(&[slime]);
    assert!(!legacy_slime_geometry_guard.vertices.is_empty());
    let legacy_magma_cube_geometry_guard = entity_model_mesh(&[magma_cube]);
    assert!(!legacy_magma_cube_geometry_guard.vertices.is_empty());
    let legacy_goat_geometry_guard = entity_model_mesh(&[goat]);
    assert!(!legacy_goat_geometry_guard.vertices.is_empty());
    let legacy_polar_bear_geometry_guard = entity_model_mesh(&[polar_bear]);
    assert!(!legacy_polar_bear_geometry_guard.vertices.is_empty());
    let legacy_hoglin_geometry_guard = entity_model_mesh(&[hoglin]);
    assert!(!legacy_hoglin_geometry_guard.vertices.is_empty());
    let legacy_zoglin_baby_geometry_guard = entity_model_mesh(&[zoglin_baby]);
    assert!(!legacy_zoglin_baby_geometry_guard.vertices.is_empty());
    let legacy_piglin_geometry_guard = entity_model_mesh(&[piglin]);
    assert!(!legacy_piglin_geometry_guard.vertices.is_empty());
    let legacy_ravager_geometry_guard = entity_model_mesh(&[ravager]);
    assert!(!legacy_ravager_geometry_guard.vertices.is_empty());
    let legacy_villager_geometry_guard = entity_model_mesh(&[villager]);
    assert!(!legacy_villager_geometry_guard.vertices.is_empty());
    let legacy_wandering_trader_geometry_guard = entity_model_mesh(&[wandering_trader]);
    assert!(!legacy_wandering_trader_geometry_guard.vertices.is_empty());
    let legacy_iron_golem_geometry_guard = entity_model_mesh(&[iron_golem]);
    assert!(!legacy_iron_golem_geometry_guard.vertices.is_empty());
    let legacy_snow_golem_geometry_guard = entity_model_mesh(&[snow_golem]);
    assert!(!legacy_snow_golem_geometry_guard.vertices.is_empty());
    let legacy_witch_geometry_guard = entity_model_mesh(&[witch]);
    assert!(!legacy_witch_geometry_guard.vertices.is_empty());
    let legacy_copper_golem_geometry_guard = entity_model_mesh(&[copper_golem]);
    assert!(!legacy_copper_golem_geometry_guard.vertices.is_empty());
    let legacy_husk_geometry_guard = entity_model_mesh(&[husk]);
    assert!(!legacy_husk_geometry_guard.vertices.is_empty());
    let legacy_drowned_geometry_guard = entity_model_mesh(&[drowned]);
    assert!(!legacy_drowned_geometry_guard.vertices.is_empty());
    let legacy_zombie_villager_geometry_guard = entity_model_mesh(&[zombie_villager]);
    assert!(!legacy_zombie_villager_geometry_guard.vertices.is_empty());
}

#[test]
fn entity_textured_shader_samples_bound_texture_and_discards_alpha() {
    for shader in [
        ENTITY_MODEL_TEXTURED_SHADER,
        ENTITY_MODEL_TEXTURED_CULL_SHADER,
    ] {
        assert!(shader.contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
        assert!(shader.contains("discard"));
    }
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x3]
    );
}

#[test]
fn entity_textured_shader_samples_dynamic_lightmap_texture() {
    for shader in [
        ENTITY_MODEL_TEXTURED_SHADER,
        ENTITY_MODEL_TEXTURED_CULL_SHADER,
    ] {
        assert!(shader.contains("@group(1) @binding(0)"));
        assert!(shader.contains("var lightmap_texture: texture_2d<f32>"));
        assert!(shader.contains("@group(1) @binding(1)"));
        assert!(shader.contains("var lightmap_sampler: sampler"));
        assert!(shader.contains("fn sample_lightmap(light: vec2<f32>) -> vec3<f32>"));
        assert!(shader.contains("light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0)"));
        assert!(shader.contains("vec2<f32>(15.5 / 16.0)"));
        assert!(shader.contains("textureSample(lightmap_texture, lightmap_sampler, uv).rgb"));
        assert!(shader.contains("let light_color = sample_lightmap(input.light)"));
        assert!(!shader.contains("fn lightmap_brightness"));
        assert!(!shader.contains("camera.lightmap_factors.y"));
    }
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("rgb * per_face_diffuse_light(input.normal, front_facing) * light_color"));
    assert!(ENTITY_MODEL_TEXTURED_CULL_SHADER
        .contains("rgb * diffuse_light(input.normal) * light_color"));
}

#[test]
fn entity_textured_shader_applies_vanilla_level_diffuse_lighting() {
    // Vanilla entity.vsh calls minecraft_mix_light with the current Lighting.Entry
    // directions, MINECRAFT_LIGHT_POWER 0.6 and ambient 0.4.
    for shader in [
        ENTITY_MODEL_TEXTURED_SHADER,
        ENTITY_MODEL_TEXTURED_CULL_SHADER,
    ] {
        assert!(shader.contains("@location(5) normal: vec3<f32>"));
        assert!(shader.contains("minecraft_light0: vec4<f32>"));
        assert!(shader.contains("minecraft_light1: vec4<f32>"));
        assert!(shader.contains("let light0 = normalize(camera.minecraft_light0.xyz)"));
        assert!(shader.contains("let light1 = normalize(camera.minecraft_light1.xyz)"));
        assert!(shader.contains("(light_value.x + light_value.y) * 0.6 + 0.4"));
    }
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("per_face_diffuse_light(input.normal, front_facing) * light_color"));
    assert!(ENTITY_MODEL_TEXTURED_CULL_SHADER.contains("diffuse_light(input.normal) * light_color"));
}

#[test]
fn entity_textured_shader_applies_vanilla_per_face_lighting() {
    // Vanilla entity.vsh's PER_FACE_LIGHTING branch computes front and back
    // vertex colors from opposite light values, and entity.fsh selects with
    // gl_FrontFacing. WGSL carries the same choice as `front_facing`.
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("@builtin(front_facing) front_facing: bool"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("fn per_face_diffuse_light(normal: vec3<f32>, front_facing: bool) -> f32"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("if (front_facing)"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("diffuse_light(-normal)"));
}

#[test]
fn entity_textured_cull_shader_uses_vanilla_single_face_lighting() {
    // Vanilla entitySolid / entityCutoutCull / entityTranslucentCull do not set
    // PER_FACE_LIGHTING; entity.vsh uses minecraft_mix_light with the submitted
    // normal once, without the gl_FrontFacing back-face branch.
    assert!(!ENTITY_MODEL_TEXTURED_CULL_SHADER.contains("@builtin(front_facing)"));
    assert!(!ENTITY_MODEL_TEXTURED_CULL_SHADER.contains("per_face_diffuse_light"));
    assert!(ENTITY_MODEL_TEXTURED_CULL_SHADER.contains("diffuse_light(input.normal)"));
    assert!(!ENTITY_MODEL_TEXTURED_CULL_SHADER.contains("diffuse_light(-normal)"));
}

#[test]
fn entity_colored_shader_samples_dynamic_lightmap_texture() {
    // Colored meshes are a renderer-owned debug fallback, but their lighting now
    // follows the same vanilla entity.vsh shape used by non-cull entityCutout:
    // submitted normal + PER_FACE_LIGHTING front/back choice + dynamic LightTexture.
    assert!(ENTITY_MODEL_SHADER.contains("@location(4) normal: vec3<f32>"));
    assert!(ENTITY_MODEL_SHADER.contains("minecraft_light0: vec4<f32>"));
    assert!(ENTITY_MODEL_SHADER.contains("minecraft_light1: vec4<f32>"));
    assert!(ENTITY_MODEL_SHADER.contains("let light0 = normalize(camera.minecraft_light0.xyz)"));
    assert!(ENTITY_MODEL_SHADER.contains("let light1 = normalize(camera.minecraft_light1.xyz)"));
    assert!(ENTITY_MODEL_SHADER.contains("(light_value.x + light_value.y) * 0.6 + 0.4"));
    assert!(ENTITY_MODEL_SHADER.contains("@builtin(front_facing) front_facing: bool"));
    assert!(ENTITY_MODEL_SHADER
        .contains("per_face_diffuse_light(input.normal, front_facing) * light_color"));
    assert!(ENTITY_MODEL_SHADER.contains("@group(1) @binding(0)"));
    assert!(ENTITY_MODEL_SHADER.contains("var lightmap_texture: texture_2d<f32>"));
    assert!(ENTITY_MODEL_SHADER.contains("@group(1) @binding(1)"));
    assert!(ENTITY_MODEL_SHADER.contains("var lightmap_sampler: sampler"));
    assert!(ENTITY_MODEL_SHADER.contains("fn sample_lightmap(light: vec2<f32>) -> vec3<f32>"));
    assert!(ENTITY_MODEL_SHADER.contains("light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0)"));
    assert!(
        ENTITY_MODEL_SHADER.contains("textureSample(lightmap_texture, lightmap_sampler, uv).rgb")
    );
    assert!(ENTITY_MODEL_SHADER.contains("let light_color = sample_lightmap(input.light)"));
    assert!(!ENTITY_MODEL_SHADER.contains("rgb * light_color"));
    assert!(ENTITY_MODEL_SHADER.contains("input.color.rgb"));
    assert!(!ENTITY_MODEL_SHADER.contains("fn lightmap_brightness"));
    assert!(!ENTITY_MODEL_SHADER.contains("camera.lightmap_factors.y"));
}

#[test]
fn entity_shaders_apply_vanilla_overlay_texture_mix() {
    // OverlayTexture: hurt row (v < 8) mixes toward red at alpha 179/255; white
    // rows mix toward white at alpha 1 - u/15 * 0.75. Applied before the
    // lightmap, matching the vanilla entity fragment shader order.
    for shader in [
        ENTITY_MODEL_SHADER,
        ENTITY_MODEL_TEXTURED_SHADER,
        ENTITY_MODEL_TEXTURED_CULL_SHADER,
    ] {
        assert!(shader.contains("input.overlay.y < 8.0"));
        assert!(shader.contains("mix(vec3<f32>(1.0, 0.0, 0.0), rgb, 179.0 / 255.0)"));
        assert!(shader.contains("1.0 - input.overlay.x / 15.0 * 0.75"));
    }
    // Eyes stay emissive and unaffected by the overlay.
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("overlay"));
}

#[test]
fn entity_scroll_shaders_split_breeze_wind_lightmap_from_energy_swirl_emissive() {
    // Vanilla RenderPipelines.BREEZE_WIND uses the entity shader with
    // NO_OVERLAY + NO_CARDINAL_LIGHTING, but still samples the lightmap.
    assert!(ENTITY_MODEL_SCROLL_SHADER.contains("@location(4) light: vec2<f32>"));
    assert!(ENTITY_MODEL_SCROLL_SHADER.contains("@group(1) @binding(0)"));
    assert!(ENTITY_MODEL_SCROLL_SHADER.contains("var lightmap_texture: texture_2d<f32>"));
    assert!(ENTITY_MODEL_SCROLL_SHADER.contains("var lightmap_sampler: sampler"));
    assert!(ENTITY_MODEL_SCROLL_SHADER
        .contains("textureSample(lightmap_texture, lightmap_sampler, uv).rgb"));
    assert!(ENTITY_MODEL_SCROLL_SHADER.contains("let light_color = sample_lightmap(input.light)"));
    assert!(ENTITY_MODEL_SCROLL_SHADER.contains("texel.rgb * light_color"));

    // Vanilla RenderPipelines.ENERGY_SWIRL defines EMISSIVE + NO_OVERLAY +
    // NO_CARDINAL_LIGHTING, so the additive scroll shader skips lightmap while
    // still applying the shared fog UBO like the entity fragment shader.
    assert!(ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("return apply_fog(texel"));
    assert!(!ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("lightmap_brightness"));
    assert!(!ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("lightmap_texture"));
    assert!(!ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("not_gamma"));
    assert!(!ENTITY_MODEL_SCROLL_EMISSIVE_SHADER.contains("texel.rgb * light_color"));
}

#[test]
fn entity_eyes_shader_samples_bound_texture_without_alpha_cutout() {
    assert!(ENTITY_MODEL_EYES_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv)"));
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x3]
    );
    // Eyes stay emissive: the lightmap shade must not dim them.
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("lightmap_brightness"));
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("not_gamma"));
}

#[test]
fn entity_model_root_transform_rotates_instances_by_body_yaw() {
    let mesh = entity_model_mesh(&[EntityModelInstance::chicken(
        26,
        [10.0, 64.0, -3.0],
        90.0,
        false,
    )]);

    let (min, max) = mesh_extents(&mesh);
    assert_close3(min, [9.5, 64.001, -3.25]);
    assert_close3(max, [10.25, 64.9385, -2.75]);
}

#[test]
fn humanoid_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        HumanoidModelFamily::Player,
        HumanoidModelFamily::Zombie,
        HumanoidModelFamily::Skeleton,
        HumanoidModelFamily::Villager,
        HumanoidModelFamily::Illager,
        HumanoidModelFamily::ArmorStand,
    ] {
        let instance = EntityModelInstance::humanoid(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn quadruped_model_families_emit_deterministic_non_empty_meshes() {
    for family in [
        QuadrupedModelFamily::Pig,
        QuadrupedModelFamily::Cow,
        QuadrupedModelFamily::Sheep,
        QuadrupedModelFamily::Horse,
        QuadrupedModelFamily::Wolf,
    ] {
        let instance = EntityModelInstance::quadruped(1, [0.0, 64.0, 0.0], 0.0, family, false);
        let mesh = entity_model_mesh(&[instance]);
        let repeat = entity_model_mesh(&[instance]);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices, repeat.vertices);
        assert_eq!(mesh.indices, repeat.indices);
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn vehicle_and_placeholder_models_emit_sane_bounds() {
    let cases = [
        EntityModelInstance::new(1, EntityModelKind::Minecart, [0.0, 64.0, 0.0], 0.0),
        EntityModelInstance::new(
            2,
            EntityModelKind::Boat {
                family: BoatModelFamily::Oak,
                chest: true,
            },
            [3.0, 64.0, 0.0],
            0.0,
        ),
        EntityModelInstance::placeholder(
            3,
            [6.0, 64.0, 0.0],
            0.0,
            "todo_test_bounds",
            1.0,
            2.0,
            0.5,
        ),
    ];

    for instance in cases {
        let mesh = entity_model_mesh(&[instance]);
        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        let (min, max) = mesh_extents(&mesh);
        assert!(max[0] > min[0]);
        assert!(max[1] > min[1]);
        assert!(max[2] > min[2]);
    }
}

#[test]
fn entity_model_kind_exposes_stable_model_keys() {
    assert_eq!(
        EntityModelKind::Chicken {
            variant: ChickenModelVariant::Temperate,
            baby: false
        }
        .model_key(),
        "chicken_temperate"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Cold,
            baby: false
        }
        .model_key(),
        "pig_cold"
    );
    assert_eq!(
        EntityModelKind::Pig {
            variant: PigModelVariant::Warm,
            baby: true
        }
        .model_key(),
        "pig_warm_baby"
    );
    assert_eq!(
        EntityModelKind::Humanoid {
            family: HumanoidModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "humanoid_zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ArmorStand {
            small: true,
            marker: false,
            show_arms: true,
            show_base_plate: false,
            pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
        }
        .model_key(),
        "armor_stand_small"
    );
    assert_eq!(EntityModelKind::Slime { size: 4 }.model_key(), "slime");
    assert_eq!(
        EntityModelKind::MagmaCube { size: 3 }.model_key(),
        "magma_cube"
    );
    assert_eq!(
        EntityModelKind::Zombie { baby: true }.model_key(),
        "zombie_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: false
        }
        .model_key(),
        "husk"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Husk,
            baby: true
        }
        .model_key(),
        "husk_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: false
        }
        .model_key(),
        "drowned"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            baby: true
        }
        .model_key(),
        "drowned_baby"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: false
        }
        .model_key(),
        "zombie_villager"
    );
    assert_eq!(
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby: true
        }
        .model_key(),
        "zombie_villager_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: false
        }
        .model_key(),
        "piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            baby: true
        }
        .model_key(),
        "piglin_baby"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::PiglinBrute,
            baby: false
        }
        .model_key(),
        "piglin_brute"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: false
        }
        .model_key(),
        "zombified_piglin"
    );
    assert_eq!(
        EntityModelKind::Piglin {
            family: PiglinModelFamily::ZombifiedPiglin,
            baby: true
        }
        .model_key(),
        "zombified_piglin_baby"
    );
    assert_eq!(EntityModelKind::Skeleton.model_key(), "skeleton");
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Stray
        }
        .model_key(),
        "stray"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Parched
        }
        .model_key(),
        "parched"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::WitherSkeleton
        }
        .model_key(),
        "wither_skeleton"
    );
    assert_eq!(
        EntityModelKind::SkeletonVariant {
            family: SkeletonModelFamily::Bogged { sheared: true }
        }
        .model_key(),
        "bogged"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Warm,
            baby: false
        }
        .model_key(),
        "cow_warm"
    );
    assert_eq!(
        EntityModelKind::Cow {
            variant: CowModelVariant::Cold,
            baby: true
        }
        .model_key(),
        "cow_cold_baby"
    );
    assert_eq!(
        EntityModelKind::Sheep {
            baby: true,
            sheared: false,
            wool_color: SheepWoolColor::White,
            jeb: false,
            age_ticks: 0.0,
        }
        .model_key(),
        "sheep_baby"
    );
    assert_eq!(
        EntityModelKind::Villager { baby: true }.model_key(),
        "villager_baby"
    );
    assert_eq!(
        EntityModelKind::WanderingTrader.model_key(),
        "wandering_trader"
    );
    assert_eq!(
        EntityModelInstance::wolf(0, [0.0, 0.0, 0.0], 0.0, true)
            .kind
            .model_key(),
        "wolf_baby"
    );
    assert_eq!(
        EntityModelKind::Horse {
            baby: true,
            variant: HorseColorVariant::White,
            markings: HorseMarkings::None
        }
        .model_key(),
        "horse_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "donkey"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "donkey_baby"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "mule"
    );
    assert_eq!(
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "mule_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false
        }
        .model_key(),
        "skeleton_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: true
        }
        .model_key(),
        "skeleton_horse_baby"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false
        }
        .model_key(),
        "zombie_horse"
    );
    assert_eq!(
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: true
        }
        .model_key(),
        "zombie_horse_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false
        }
        .model_key(),
        "camel"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: true
        }
        .model_key(),
        "camel_baby"
    );
    assert_eq!(
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            baby: true
        }
        .model_key(),
        "camel_husk"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::Creamy,
            baby: false,
            has_chest: true
        }
        .model_key(),
        "llama_creamy"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::Llama,
            variant: LlamaVariant::White,
            baby: true,
            has_chest: false
        }
        .model_key(),
        "llama_white_baby"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Brown,
            baby: false,
            has_chest: false
        }
        .model_key(),
        "trader_llama_brown"
    );
    assert_eq!(
        EntityModelKind::Llama {
            family: LlamaModelFamily::TraderLlama,
            variant: LlamaVariant::Gray,
            baby: true,
            has_chest: true
        }
        .model_key(),
        "trader_llama_gray_baby"
    );
    assert_eq!(
        EntityModelKind::Goat {
            baby: true,
            left_horn: false,
            right_horn: true
        }
        .model_key(),
        "goat_baby"
    );
    assert_eq!(EntityModelKind::Spider.model_key(), "spider");
    assert_eq!(EntityModelKind::CaveSpider.model_key(), "cave_spider");
    assert_eq!(EntityModelKind::Enderman.model_key(), "enderman");
    assert_eq!(
        EntityModelKind::IronGolem {
            crackiness: IronGolemCrackiness::None,
        }
        .model_key(),
        "iron_golem"
    );
    assert_eq!(EntityModelKind::SnowGolem.model_key(), "snow_golem");
    assert_eq!(EntityModelKind::Witch.model_key(), "witch");
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Evoker
        }
        .model_key(),
        "evoker"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
        .model_key(),
        "illusioner"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
        .model_key(),
        "pillager"
    );
    assert_eq!(
        EntityModelKind::Illager {
            family: IllagerModelFamily::Vindicator
        }
        .model_key(),
        "vindicator"
    );
    assert_eq!(
        EntityModelKind::Placeholder {
            name: "todo_test_bounds",
            bounds: EntityModelBounds {
                width: 1.0,
                height: 1.0,
                depth: 1.0
            }
        }
        .model_key(),
        "todo_test_bounds"
    );
}

#[test]
fn sanitize_entity_model_instances_drops_non_finite_instances() {
    assert_eq!(
        sanitize_entity_model_instances(vec![
            EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false),
            EntityModelInstance::chicken(2, [0.0, f32::NAN, 0.0], 0.0, false),
            EntityModelInstance::chicken(3, [0.0, 0.0, 0.0], f32::INFINITY, false),
        ]),
        vec![EntityModelInstance::chicken(1, [0.0, 0.0, 0.0], 0.0, false)]
    );
}

#[test]
fn entity_mesh_fills_per_instance_packed_light() {
    // pack(block 10, sky 0) -> shader light [10/15, 0].
    let dim = EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "dim", 1.0, 1.0, 1.0)
        .with_light_coords(10 << 4);
    // pack(block 0, sky 15) -> shader light [0, 1].
    let lit = EntityModelInstance::placeholder(2, [4.0, 0.0, 0.0], 0.0, "lit", 1.0, 1.0, 1.0)
        .with_light_coords(15 << 20);

    let mesh = entity_model_mesh(&[dim, lit]);
    assert!(!mesh.vertices.is_empty());
    // Every vertex carries one of the two per-instance lights, and both appear:
    // the post-pass assigned each entity's geometry its own sampled light.
    let dim_light = [10.0 / 15.0, 0.0];
    let lit_light = [0.0, 1.0];
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.light == dim_light || vertex.light == lit_light));
    assert!(mesh.vertices.iter().any(|vertex| vertex.light == dim_light));
    assert!(mesh.vertices.iter().any(|vertex| vertex.light == lit_light));
}

#[test]
fn entity_mesh_fills_per_instance_hurt_overlay() {
    let calm = EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "calm", 1.0, 1.0, 1.0);
    let hurt = EntityModelInstance::placeholder(2, [4.0, 0.0, 0.0], 0.0, "hurt", 1.0, 1.0, 1.0)
        .with_has_red_overlay(true);

    let mesh = entity_model_mesh(&[calm, hurt]);
    assert!(!mesh.vertices.is_empty());
    // Calm entities carry OverlayTexture NO_OVERLAY = [0, 10]; hurt entities the
    // red row [0, 3]. Both appear: each entity got its own overlay coords.
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.overlay == [0.0, 10.0] || vertex.overlay == [0.0, 3.0]));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == [0.0, 10.0]));
    assert!(mesh
        .vertices
        .iter()
        .any(|vertex| vertex.overlay == [0.0, 3.0]));
}

#[test]
fn colored_runtime_mesh_applies_vanilla_force_transparent_alpha() {
    // Vanilla `LivingEntityRenderer.submit` uses the force-transparent branch
    // when `!isBodyVisible && !isInvisibleToPlayer`, multiplying the model tint
    // by `0x26ffffff` and submitting `entityTranslucentCullItemTarget`.
    let hidden = EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "hidden", 1.0, 1.0, 1.0)
        .with_invisible(true);
    assert!(entity_model_colored_runtime_mesh(&[hidden])
        .vertices
        .is_empty());

    let self_visible =
        EntityModelInstance::placeholder(2, [0.0, 0.0, 0.0], 0.0, "self_visible", 1.0, 1.0, 1.0)
            .with_invisible(true)
            .with_invisible_to_player(false)
            .with_light_coords((6_u32 << 4) | (8_u32 << 20))
            .with_has_red_overlay(true);
    let mesh = entity_model_colored_runtime_mesh(&[self_visible]);
    assert!(!mesh.vertices.is_empty());
    for vertex in &mesh.vertices {
        assert_eq!(vertex.color[3], 38.0 / 255.0);
        assert_eq!(vertex.light, [6.0 / 15.0, 8.0 / 15.0]);
        assert_eq!(vertex.overlay, [0.0, 3.0]);
    }

    let visible =
        EntityModelInstance::placeholder(3, [0.0, 0.0, 0.0], 0.0, "visible", 1.0, 1.0, 1.0);
    let visible_mesh = entity_model_colored_runtime_mesh(&[visible]);
    assert!(visible_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.color[3] == 1.0));
}

#[test]
fn colored_runtime_mesh_applies_vanilla_outline_color_for_hidden_glowing() {
    // Vanilla `LivingEntityRenderer.getRenderType` returns `RenderTypes.outline`
    // only when the body is invisible to this client and `appearsGlowing()`.
    let hidden_glowing =
        EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "hidden_glowing", 1.0, 1.0, 1.0)
            .with_invisible(true)
            .with_outline_color(0xff33_66cc)
            .with_light_coords((3_u32 << 4) | (12_u32 << 20))
            .with_has_red_overlay(true);
    let mesh = entity_model_colored_runtime_mesh(&[hidden_glowing]);
    assert!(!mesh.vertices.is_empty());
    let outline_tint = [
        0x33 as f32 / 255.0,
        0x66 as f32 / 255.0,
        0xcc as f32 / 255.0,
        1.0,
    ];
    for vertex in &mesh.vertices {
        assert_eq!(vertex.color, outline_tint);
        assert_eq!(vertex.light, [3.0 / 15.0, 12.0 / 15.0]);
        assert_eq!(vertex.overlay, [0.0, 3.0]);
    }

    let visible_glowing =
        EntityModelInstance::placeholder(2, [0.0, 0.0, 0.0], 0.0, "visible_glowing", 1.0, 1.0, 1.0)
            .with_outline_color(0xff33_66cc);
    let visible_mesh = entity_model_colored_runtime_mesh(&[visible_glowing]);
    assert!(!visible_mesh.vertices.is_empty());
    assert!(visible_mesh
        .vertices
        .iter()
        .all(|vertex| vertex.color != outline_tint));
}

#[test]
fn entity_model_vertex_layout_matches_shader_inputs() {
    let layout = entity_model_vertex_layout();

    assert_eq!(
        layout.array_stride,
        std::mem::size_of::<EntityModelVertex>() as wgpu::BufferAddress
    );
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES.len(), 5);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[0].shader_location, 0);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[1].shader_location, 1);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[2].shader_location, 2);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[3].shader_location, 3);
    assert_eq!(ENTITY_MODEL_VERTEX_ATTRIBUTES[4].shader_location, 4);
}

#[test]
fn entity_colored_fallback_defers_face_shade_to_shader_normals() {
    let instance =
        EntityModelInstance::placeholder(1, [0.0, 0.0, 0.0], 0.0, "debug", 1.0, 1.0, 1.0);
    let mesh = entity_model_mesh(&[instance]);

    assert!(!mesh.vertices.is_empty());
    assert!(mesh
        .vertices
        .iter()
        .all(|vertex| vertex.color == PLACEHOLDER_COLOR));
    for normal in [
        [0.0, -1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    ] {
        let expected = Vec3::from_array(normal);
        assert!(mesh
            .vertices
            .iter()
            .any(|vertex| Vec3::from_array(vertex.normal).dot(expected) > 0.999));
    }
}

fn mesh_extents(mesh: &EntityModelMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn textured_mesh_extents(mesh: &EntityModelTexturedMesh) -> ([f32; 3], [f32; 3]) {
    let mut vertices = mesh.vertices.iter();
    let first = vertices.next().expect("mesh has vertices").position;
    let mut min = Vec3::from_array(first);
    let mut max = Vec3::from_array(first);
    for vertex in vertices {
        let position = Vec3::from_array(vertex.position);
        min = min.min(position);
        max = max.max(position);
    }
    (min.to_array(), max.to_array())
}

fn assert_close3(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn assert_close2(actual: [f32; 2], expected: [f32; 2]) {
    for (actual, expected) in actual.iter().copied().zip(expected.iter().copied()) {
        assert!(
            (actual - expected).abs() < 1.0e-4,
            "expected {expected}, got {actual}"
        );
    }
}

fn spider_texture_images() -> Vec<EntityModelTextureImage> {
    spider_entity_texture_refs()
        .iter()
        .enumerate()
        .map(|(index, texture)| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![index as u8; len])
        })
        .collect()
}

fn assert_same_geometry(actual: &EntityModelMesh, expected: &EntityModelMesh) {
    assert_eq!(actual.opaque_faces, expected.opaque_faces);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.vertices.len(), expected.vertices.len());
    for (actual, expected) in actual.vertices.iter().zip(expected.vertices.iter()) {
        assert_eq!(actual.position, expected.position);
    }
}

/// Asserts two meshes share the same body-layer model structure (cube count, faces, and
/// indices) without requiring byte-identical vertex positions — used when two entities reuse
/// the same model but animate it differently (e.g. the zombified piglin defers its arm pose
/// to rest while the regular piglin applies the always-on idle arm bob).
fn assert_same_structure(actual: &EntityModelMesh, expected: &EntityModelMesh) {
    assert_eq!(actual.opaque_faces, expected.opaque_faces);
    assert_eq!(actual.indices, expected.indices);
    assert_eq!(actual.vertices.len(), expected.vertices.len());
}

fn assert_part(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert!(part.children.is_empty());
}

fn assert_part_tree(
    part: &ModelPartDesc,
    offset: [f32; 3],
    rotation: [f32; 3],
    cubes: &[ModelCubeDesc],
    children: &[ModelPartDesc],
) {
    assert_eq!(part.pose.offset, offset);
    assert_eq!(part.pose.rotation, rotation);
    assert_eq!(part.cubes, cubes);
    assert_eq!(part.children, children);
}
