use super::*;
use crate::camera::{CameraUniform, LevelLighting, LightmapEnvironment};
use crate::entity_models::equipment::ResolvedEntityEquipmentAssetCatalog;
use crate::entity_models::gpu::build_entity_model_texture_atlas_with_height_limit;
use crate::entity_models::model::ModelPart;
use crate::entity_models::model_layer::ModelLayer;
use crate::terrain::TerrainUvRect;

pub(super) fn texture_refs_matching(
    predicate: impl Fn(&str) -> bool,
) -> Vec<EntityModelTextureRef> {
    entity_model_texture_refs()
        .iter()
        .copied()
        .filter(|texture| predicate(texture.path))
        .collect()
}

const TEST_EQUIPMENT_GENERATION: u64 = 9_001;
const TEST_IRON_EQUIPMENT_INDEX: u32 = 0;
const TEST_LEATHER_EQUIPMENT_INDEX: u32 = 1;
const TEST_NETHERITE_EQUIPMENT_INDEX: u32 = 2;
const TEST_SADDLE_EQUIPMENT_INDEX: u32 = 3;
const TEST_WINGS_EQUIPMENT_INDEX: u32 = 4;
const TEST_WOLF_EQUIPMENT_INDEX: u32 = 5;
const TEST_HAPPY_GHAST_WHITE_EQUIPMENT_INDEX: u32 = 6;
const TEST_HAPPY_GHAST_BLACK_EQUIPMENT_INDEX: u32 = 7;
const TEST_HAPPY_GHAST_BLUE_EQUIPMENT_INDEX: u32 = 8;
const TEST_LLAMA_WHITE_EQUIPMENT_INDEX: u32 = 9;
const TEST_LLAMA_BLACK_EQUIPMENT_INDEX: u32 = 10;
const TEST_LLAMA_TRADER_EQUIPMENT_INDEX: u32 = 11;
const TEST_LLAMA_TRADER_BABY_EQUIPMENT_INDEX: u32 = 12;

fn test_equipment_asset_id(index: u32) -> EntityEquipmentAssetId {
    EntityEquipmentAssetId::new(TEST_EQUIPMENT_GENERATION, index)
}

fn test_iron_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_IRON_EQUIPMENT_INDEX)
}

fn test_leather_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_LEATHER_EQUIPMENT_INDEX)
}

fn test_netherite_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_NETHERITE_EQUIPMENT_INDEX)
}

fn test_saddle_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_SADDLE_EQUIPMENT_INDEX)
}

fn test_wings_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_WINGS_EQUIPMENT_INDEX)
}

fn test_wolf_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_WOLF_EQUIPMENT_INDEX)
}

fn test_happy_ghast_white_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_HAPPY_GHAST_WHITE_EQUIPMENT_INDEX)
}

fn test_happy_ghast_black_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_HAPPY_GHAST_BLACK_EQUIPMENT_INDEX)
}

fn test_happy_ghast_blue_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_HAPPY_GHAST_BLUE_EQUIPMENT_INDEX)
}

fn test_llama_white_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_LLAMA_WHITE_EQUIPMENT_INDEX)
}

fn test_llama_black_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_LLAMA_BLACK_EQUIPMENT_INDEX)
}

fn test_llama_trader_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_LLAMA_TRADER_EQUIPMENT_INDEX)
}

fn test_llama_trader_baby_equipment_asset_id() -> EntityEquipmentAssetId {
    test_equipment_asset_id(TEST_LLAMA_TRADER_BABY_EQUIPMENT_INDEX)
}

fn test_static_equipment_layer(
    texture: EntityModelTextureRef,
    dyeable: Option<EntityEquipmentDyeable>,
    use_player_texture: bool,
) -> EntityEquipmentAssetLayer {
    EntityEquipmentAssetLayer {
        texture: EntityEquipmentAssetLayerTexture::Static(texture),
        dyeable,
        use_player_texture,
    }
}

fn test_static_equipment_asset(
    layers: impl IntoIterator<Item = (EntityEquipmentLayerType, Vec<EntityEquipmentAssetLayer>)>,
) -> EntityEquipmentAsset {
    EntityEquipmentAsset {
        layers: layers.into_iter().collect(),
    }
}

fn test_static_equipment_catalog() -> ResolvedEntityEquipmentAssetCatalog {
    let plain = |texture| test_static_equipment_layer(texture, None, false);
    let dyed = |texture, color_when_undyed| {
        test_static_equipment_layer(
            texture,
            Some(EntityEquipmentDyeable { color_when_undyed }),
            false,
        )
    };
    let humanoid_asset = |humanoid, leggings, baby, horse_body, nautilus_body| {
        test_static_equipment_asset([
            (EntityEquipmentLayerType::Humanoid, vec![plain(humanoid)]),
            (
                EntityEquipmentLayerType::HumanoidLeggings,
                vec![plain(leggings)],
            ),
            (EntityEquipmentLayerType::HumanoidBaby, vec![plain(baby)]),
            (EntityEquipmentLayerType::HorseBody, horse_body),
            (EntityEquipmentLayerType::NautilusBody, nautilus_body),
        ])
    };
    let saddle = test_static_equipment_asset([
        (
            EntityEquipmentLayerType::PigSaddle,
            vec![plain(PIG_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::StriderSaddle,
            vec![plain(STRIDER_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::CamelSaddle,
            vec![plain(CAMEL_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::CamelHuskSaddle,
            vec![plain(CAMEL_HUSK_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::HorseSaddle,
            vec![plain(HORSE_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::DonkeySaddle,
            vec![plain(DONKEY_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::MuleSaddle,
            vec![plain(MULE_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::ZombieHorseSaddle,
            vec![plain(ZOMBIE_HORSE_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::SkeletonHorseSaddle,
            vec![plain(SKELETON_HORSE_SADDLE_TEXTURE_REF)],
        ),
        (
            EntityEquipmentLayerType::NautilusSaddle,
            vec![plain(NAUTILUS_SADDLE_TEXTURE_REF)],
        ),
    ]);
    let assets = vec![
        humanoid_asset(
            ARMOR_IRON_HUMANOID_TEXTURE_REF,
            ARMOR_IRON_LEGGINGS_TEXTURE_REF,
            ARMOR_IRON_BABY_HUMANOID_TEXTURE_REF,
            vec![plain(HORSE_BODY_IRON_TEXTURE_REF)],
            vec![plain(NAUTILUS_BODY_IRON_TEXTURE_REF)],
        ),
        test_static_equipment_asset([
            (
                EntityEquipmentLayerType::Humanoid,
                vec![dyed(ARMOR_LEATHER_HUMANOID_TEXTURE_REF, Some(0x00A0_6540))],
            ),
            (
                EntityEquipmentLayerType::HumanoidLeggings,
                vec![dyed(ARMOR_LEATHER_LEGGINGS_TEXTURE_REF, Some(0x00A0_6540))],
            ),
            (
                EntityEquipmentLayerType::HumanoidBaby,
                vec![dyed(
                    ARMOR_LEATHER_BABY_HUMANOID_TEXTURE_REF,
                    Some(0x00A0_6540),
                )],
            ),
            (
                EntityEquipmentLayerType::HorseBody,
                vec![
                    dyed(HORSE_BODY_LEATHER_TEXTURE_REF, Some(0x00A0_6540)),
                    plain(HORSE_BODY_LEATHER_OVERLAY_TEXTURE_REF),
                ],
            ),
            (
                EntityEquipmentLayerType::NautilusBody,
                vec![plain(NAUTILUS_BODY_IRON_TEXTURE_REF)],
            ),
        ]),
        humanoid_asset(
            ARMOR_NETHERITE_HUMANOID_TEXTURE_REF,
            ARMOR_NETHERITE_LEGGINGS_TEXTURE_REF,
            ARMOR_NETHERITE_BABY_HUMANOID_TEXTURE_REF,
            vec![plain(HORSE_BODY_NETHERITE_TEXTURE_REF)],
            vec![plain(NAUTILUS_BODY_NETHERITE_TEXTURE_REF)],
        ),
        saddle,
        test_static_equipment_asset([(
            EntityEquipmentLayerType::Wings,
            vec![test_static_equipment_layer(
                ELYTRA_EQUIPMENT_WINGS_TEXTURE_REF,
                None,
                true,
            )],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::WolfBody,
            vec![
                plain(WOLF_BODY_ARMADILLO_SCUTE_TEXTURE_REF),
                dyed(WOLF_BODY_ARMADILLO_SCUTE_OVERLAY_TEXTURE_REF, None),
            ],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::HappyGhastBody,
            vec![plain(HAPPY_GHAST_HARNESS_WHITE_TEXTURE_REF)],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::HappyGhastBody,
            vec![plain(HAPPY_GHAST_HARNESS_BLACK_TEXTURE_REF)],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::HappyGhastBody,
            vec![plain(HAPPY_GHAST_HARNESS_BLUE_TEXTURE_REF)],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::LlamaBody,
            vec![plain(LLAMA_BODY_WHITE_TEXTURE_REF)],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::LlamaBody,
            vec![plain(LLAMA_BODY_BLACK_TEXTURE_REF)],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::LlamaBody,
            vec![plain(LLAMA_BODY_TRADER_TEXTURE_REF)],
        )]),
        test_static_equipment_asset([(
            EntityEquipmentLayerType::LlamaBody,
            vec![plain(LLAMA_BODY_TRADER_BABY_TEXTURE_REF)],
        )]),
    ];
    EntityEquipmentAssetCatalog::new(TEST_EQUIPMENT_GENERATION, assets)
        .resolve(&[])
        .unwrap()
}

fn entity_model_textured_meshes_with_static_equipment_fixture(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
) -> EntityModelTexturedMeshes {
    let equipment_assets = test_static_equipment_catalog();
    entity_model_textured_meshes_with_equipment_for_camera(
        instances,
        atlas,
        &[],
        &equipment_assets,
        None,
        None,
        None,
        None,
    )
}

fn entity_model_textured_meshes_with_static_equipment_and_dynamic_textures_fixture(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) -> EntityModelTexturedMeshes {
    let equipment_assets = test_static_equipment_catalog();
    entity_model_textured_meshes_with_equipment_for_camera(
        instances,
        atlas,
        &[],
        &equipment_assets,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        None,
        None,
    )
}

fn test_block_entity_crumbling(origin: [f32; 3]) -> BlockEntityCrumbling {
    BlockEntityCrumbling {
        origin,
        uv: TerrainUvRect {
            min: [0.25, 0.375],
            max: [0.375, 0.5],
        },
    }
}

fn expected_equipment_wings_pass(texture: EntityModelTextureRef) -> EntityModelLayerPass {
    equipment_layer_pass(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::ArmorCutoutNoCull,
        texture,
        [1.0; 4],
        0,
    )
}

mod allay;
mod armadillo;
mod armor;
mod armor_stand;
mod arrow;
mod axolotl;
mod banner;
mod bat;
mod beacon;
mod bed;
mod bee;
mod bell;
mod blaze;
mod boat;
mod book;
mod breeze;
mod camel;
mod chest;
mod chicken;
mod cod;
mod conduit;
mod copper_golem;
mod cow;
mod creaking;
mod creeper;
mod custom_head_skull;
mod death;
mod decorated_pot;
mod dolphin;
mod dragon_fireball;
mod end_crystal;
mod end_portal;
mod ender_dragon;
mod enderman;
mod endermite;
mod equine;
mod equipment_assets;
mod evoker_fangs;
mod experience_orb;
mod feline;
mod fishing_hook;
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
mod leash;
mod leash_knot;
mod llama;
mod llama_spit;
mod minecart;
mod model;
mod mooshroom;
mod nautilus;
mod painting;
mod panda;
mod parrot;
mod phantom;
mod pig;
mod piglin;
mod player;
mod player_stuck;
mod polar_bear;
mod pufferfish;
mod rabbit;
mod ravager;
mod salmon;
mod sheep;
mod shulker;
mod shulker_box;
mod shulker_bullet;
mod sign;
mod silverfish;
mod skeleton;
mod skull_block;
mod slime;
mod sniffer;
mod spider;
mod squid;
mod strider;
mod tadpole;
mod trident;
mod tropical_fish;
mod turtle;
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
fn model_texture_atlas_separates_pack_resolution_from_logical_uv_extent() {
    // GhastModel.createBodyLayer declares a logical 64x32 UV extent while
    // Vanilla 26.1 ships 128x64 ghast PNGs. The full physical image occupies
    // the atlas rect without changing the model-facing logical coordinates.
    let texture = EntityModelTextureRef {
        path: "textures/entity/ghast/ghast.png",
        size: [64, 32],
    };
    let image =
        EntityModelTextureImage::with_physical_size(texture, [128, 64], vec![0x7f; 128 * 64 * 4]);

    let (atlas, rgba) = build_entity_model_texture_atlas(&[image]).unwrap();

    assert_eq!([atlas.width, atlas.height], [128, 64]);
    assert_eq!(atlas.entries[0].texture, texture);
    assert_eq!(atlas.entries[0].uv.min, [0.0, 0.0]);
    assert_eq!(atlas.entries[0].uv.max, [1.0, 1.0]);
    assert_eq!(rgba.len(), 128 * 64 * 4);
}

#[test]
fn model_texture_atlas_wraps_physical_images_at_the_device_height_limit() {
    let images = [
        ("textures/entity/test/first.png", 0x11),
        ("textures/entity/test/second.png", 0x22),
        ("textures/entity/test/third.png", 0x33),
    ]
    .map(|(path, fill)| {
        EntityModelTextureImage::with_physical_size(
            EntityModelTextureRef { path, size: [4, 2] },
            [4, 2],
            vec![fill; 4 * 2 * 4],
        )
    });

    let (atlas, rgba) = build_entity_model_texture_atlas_with_height_limit(&images, 5).unwrap();

    assert_eq!([atlas.width, atlas.height], [8, 4]);
    assert_eq!(atlas.entries[0].uv.min, [0.0, 0.0]);
    assert_eq!(atlas.entries[0].uv.max, [0.5, 0.5]);
    assert_eq!(atlas.entries[1].uv.min, [0.0, 0.5]);
    assert_eq!(atlas.entries[1].uv.max, [0.5, 1.0]);
    assert_eq!(atlas.entries[2].uv.min, [0.5, 0.0]);
    assert_eq!(atlas.entries[2].uv.max, [1.0, 0.5]);
    assert_eq!(model_atlas_pixel(&rgba, atlas.width, 0, 0), [0x11; 4]);
    assert_eq!(model_atlas_pixel(&rgba, atlas.width, 0, 2), [0x22; 4]);
    assert_eq!(model_atlas_pixel(&rgba, atlas.width, 4, 0), [0x33; 4]);
    assert_eq!(model_atlas_pixel(&rgba, atlas.width, 4, 2), [0; 4]);
}

fn model_atlas_pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let offset = usize::try_from((y * width + x) * 4).unwrap();
    rgba[offset..offset + 4].try_into().unwrap()
}

#[test]
fn textured_layer_render_type_names_match_vanilla_render_types() {
    let cases = [
        (
            EntityModelLayerRenderType::EntitySolid,
            "entitySolid",
            EntityModelLayerRenderBucket::Solid,
            true,
            true,
            false,
        ),
        (
            EntityModelLayerRenderType::EntitySolidZOffsetForward,
            "entitySolidZOffsetForward",
            EntityModelLayerRenderBucket::SolidZOffsetForward,
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
            EntityModelLayerRenderType::ArmorTrimCutoutNoCull,
            "armorTrimsSheet",
            EntityModelLayerRenderBucket::Cutout,
            true,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull,
            "armorTrimsDecalSheet",
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
            EntityModelLayerRenderType::EntityCutoutDissolve,
            "entityCutoutDissolve",
            EntityModelLayerRenderBucket::Dissolve,
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
            EntityModelLayerRenderBucket::CutoutZOffset,
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
            EntityModelLayerRenderType::BannerPattern,
            "bannerPattern",
            EntityModelLayerRenderBucket::Translucent,
            false,
            true,
            true,
        ),
        (
            EntityModelLayerRenderType::EntityTranslucentEmissive,
            "entityTranslucentEmissive",
            EntityModelLayerRenderBucket::TranslucentEmissive,
            false,
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
            EntityModelLayerRenderType::ArmorEntityGlint,
            "armorEntityGlint",
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
            EntityModelLayerRenderBucket::EndCrystalBeam,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::EndPortal,
            "end_portal",
            EntityModelLayerRenderBucket::Portal,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::EndGateway,
            "end_gateway",
            EntityModelLayerRenderBucket::Portal,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::BeaconBeamOpaque,
            "beacon_beam",
            EntityModelLayerRenderBucket::BeaconBeamOpaque,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::BeaconBeamTranslucent,
            "beacon_beam",
            EntityModelLayerRenderBucket::BeaconBeamTranslucent,
            false,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::DragonRays,
            "dragonRays",
            EntityModelLayerRenderBucket::PositionColor,
            false,
            false,
            true,
        ),
        (
            EntityModelLayerRenderType::DragonRaysDepth,
            "dragonRaysDepth",
            EntityModelLayerRenderBucket::DepthOnly,
            false,
            false,
            false,
        ),
        (
            EntityModelLayerRenderType::Leash,
            "leash",
            EntityModelLayerRenderBucket::PositionColor,
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
        assert_eq!(
            render_type.affects_crumbling(),
            matches!(
                render_type,
                EntityModelLayerRenderType::EntitySolid
                    | EntityModelLayerRenderType::EntitySolidZOffsetForward
                    | EntityModelLayerRenderType::ArmorCutoutNoCull
                    | EntityModelLayerRenderType::ArmorTrimCutoutNoCull
                    | EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
                    | EntityModelLayerRenderType::ArmorTranslucent
                    | EntityModelLayerRenderType::EntityCutout
                    | EntityModelLayerRenderType::EntityCutoutDissolve
                    | EntityModelLayerRenderType::EntityCutoutCull
                    | EntityModelLayerRenderType::EntityCutoutZOffset
                    | EntityModelLayerRenderType::EntityTranslucent
                    | EntityModelLayerRenderType::EntityTranslucentEmissive
                    | EntityModelLayerRenderType::EntityTranslucentCullItemTarget
            ),
            "{} crumbling classification",
            render_type.vanilla_name()
        );
    }
}

#[test]
fn texture_backed_blended_draw_ranges_sort_by_order_then_camera_distance() {
    let near = EntityModelInstance::sheep(701, [2.0, 64.0, 0.0], 0.0, false)
        .with_invisible(true)
        .with(|s| s.invisible_to_player = false);
    let far = EntityModelInstance::sheep(702, [10.0, 64.0, 0.0], 0.0, false)
        .with_invisible(true)
        .with(|s| s.invisible_to_player = false);
    let images: Vec<_> = [
        SHEEP_TEXTURE_REF,
        SHEEP_BABY_TEXTURE_REF,
        SHEEP_WOOL_UNDERCOAT_TEXTURE_REF,
        SHEEP_WOOL_TEXTURE_REF,
        SHEEP_WOOL_BABY_TEXTURE_REF,
    ]
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
        None,
    );
    let far_only = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[far],
        &atlas,
        None,
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );
    assert_eq!(
        near_only.item_entity_translucent_cull.vertices.len(),
        far_only.item_entity_translucent_cull.vertices.len()
    );

    for (camera, expected) in [
        ([0.0, 64.0, 0.0], [&far_only, &near_only]),
        ([20.0, 64.0, 0.0], [&near_only, &far_only]),
    ] {
        let sorted = entity_model_textured_meshes_with_dynamic_textures_for_camera(
            &[near, far],
            &atlas,
            None,
            None,
            Some(camera),
            None,
        );
        assert_eq!(sorted.sorted_item_entity_draws.len(), 2);
        assert!(
            sorted.sorted_item_entity_draws[0].sort_key.distance_sq
                > sorted.sorted_item_entity_draws[1].sort_key.distance_sq
        );
        for (draw, expected) in sorted.sorted_item_entity_draws.iter().zip(expected) {
            assert_eq!(
                (
                    draw.atlas,
                    draw.render_type,
                    draw.surface_cull,
                    draw.sort_key.order
                ),
                (
                    EntityModelTexturedDrawAtlas::Static,
                    EntityModelLayerRenderType::EntityTranslucentCullItemTarget,
                    true,
                    0
                )
            );
            let start = draw.index_start as usize;
            let end = start + draw.index_count as usize;
            assert!(sorted.item_entity_translucent_cull.indices[start..end]
                .iter()
                .map(|index| &sorted.item_entity_translucent_cull.vertices[*index as usize])
                .eq(
                    expected
                        .item_entity_translucent_cull
                        .indices
                        .iter()
                        .map(|index| &expected.item_entity_translucent_cull.vertices
                            [*index as usize])
                ));
        }
    }
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
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(
            EntityPlayerSkin::ProfiledDefault(EntityDefaultPlayerSkin::WideSteve),
        ))
    });
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
    .with(|s| {
        s.custom_head_skull = Some(EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
            dynamic_skin,
        )))
    });
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
            standalone_sampling: None,
            rgba: vec![0xdd; 64 * 64 * 4],
        }])
        .unwrap();

    let meshes = entity_model_textured_meshes_with_dynamic_textures_for_camera(
        &[static_head, dynamic_head],
        &atlas,
        Some(&dynamic_atlas),
        None,
        Some([0.0, 64.0, 0.0]),
        None,
    );

    let draws = meshes
        .sorted_main_translucent_draws
        .iter()
        .filter_map(|draw| match draw {
            EntityModelTranslucentDrawRange::Textured(draw) => Some(draw),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(draws.len(), 2);
    assert_eq!(
        draws.iter().map(|draw| draw.atlas).collect::<Vec<_>>(),
        vec![
            EntityModelTexturedDrawAtlas::DynamicPlayerSkin,
            EntityModelTexturedDrawAtlas::Static
        ]
    );
    assert!(draws.iter().all(|draw| !draw.surface_cull));
    assert_eq!(
        draws[0].index_count,
        meshes.dynamic_player_skin_translucent.indices.len() as u32
    );
    assert_eq!(
        draws[1].index_count,
        meshes.translucent.indices.len() as u32
    );
    assert_eq!(draws[0].index_start, 0);
    assert_eq!(draws[1].index_start, 0);
}

#[test]
fn equipment_layer_pass_records_vanilla_model_layer_metadata() {
    let cases = [
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            PIG_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            STRIDER_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            CAMEL_HUSK_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            NAUTILUS_BODY_IRON_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            NAUTILUS_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            LLAMA_BODY_TRADER_BABY_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            1,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            WOLF_BODY_ARMADILLO_SCUTE_OVERLAY_TEXTURE_REF,
            [
                0x33 as f32 / 255.0,
                0x66 as f32 / 255.0,
                0x99 as f32 / 255.0,
                1.0,
            ],
            2,
        ),
        (
            EntityModelLayerRenderType::ArmorEntityGlint,
            ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            2,
        ),
        (
            EntityModelLayerRenderType::ArmorTranslucent,
            WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            3,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            HORSE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            2,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            DONKEY_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            MULE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            SKELETON_HORSE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            ZOMBIE_HORSE_SADDLE_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            HORSE_BODY_LEATHER_TEXTURE_REF,
            [
                0x3F as f32 / 255.0,
                0x6C as f32 / 255.0,
                0xDA as f32 / 255.0,
                1.0,
            ],
            2,
        ),
        (
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            HORSE_BODY_IRON_TEXTURE_REF,
            [1.0, 1.0, 1.0, 1.0],
            0,
        ),
    ];

    for (render_type, texture, tint, order) in cases {
        let pass = equipment_layer_pass(
            EntityModelLayerKind::NoOverlay,
            render_type,
            texture,
            tint,
            order,
        );
        assert_eq!(pass.render_type, render_type);
        assert_eq!(
            pass.render_type.vanilla_name(),
            match render_type {
                EntityModelLayerRenderType::ArmorCutoutNoCull => "armorCutoutNoCull",
                EntityModelLayerRenderType::ArmorEntityGlint => "armorEntityGlint",
                EntityModelLayerRenderType::ArmorTranslucent => "armorTranslucent",
                _ => panic!("unexpected equipment render type"),
            }
        );
        assert_eq!(pass.texture, texture);
        assert_eq!(pass.visibility, EntityModelLayerVisibility::All);
        assert_eq!(pass.tint, tint);
        assert_eq!(pass.order, order);
    }
}

#[test]
fn entity_textured_shader_samples_bound_texture_and_discards_alpha() {
    let shader = ENTITY_MODEL_TEXTURED_SHADER;
    assert!(shader.contains("let sample = sample_entity_texture("));
    assert!(shader.contains(
        "let atlas_sample = textureSample(entity_texture_atlas, entity_sampler, atlas_uv)"
    ));
    assert!(shader.contains("return textureLoad(entity_texture_atlas, pixel_origin + resolved, 0)"));
    assert!(shader.contains("if sample.a < 0.1"));
    assert!(shader.contains("let texel = sample * input.tint"));
    assert!(!shader.contains("if texel.a <= 0.1"));
    assert!(shader.contains("discard"));
    assert_eq!(
        ENTITY_MODEL_TEXTURED_VERTEX_ATTRIBUTES,
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x3, 6 => Float32x2, 7 => Float32x2, 8 => Float32x4]
    );
}

#[test]
fn entity_textured_shader_bounds_resource_texture_sampling_to_its_entry() {
    let shader = ENTITY_MODEL_TEXTURED_SHADER;
    assert!(shader.contains("return ((coordinate % extent) + extent) % extent"));
    assert!(shader.contains("return clamp(coordinate, 0, extent - 1)"));
    assert!(shader.contains("let coordinate = vec2<i32>(floor(local_uv * size_f))"));
    assert!(shader.contains("let sample_position = local_uv * size_f - vec2<f32>(0.5)"));
    assert_eq!(
        shader.matches("load_standalone_texel(lower").count(),
        4,
        "linear sampling must resolve all four taps inside the entry"
    );
}

#[test]
fn entity_textured_shader_samples_dynamic_lightmap_texture() {
    let shader = ENTITY_MODEL_TEXTURED_SHADER;
    assert!(shader.contains("@group(1) @binding(0)"));
    assert!(shader.contains("var lightmap_texture: texture_2d<f32>"));
    assert!(shader.contains("@group(1) @binding(1)"));
    assert!(shader.contains("var lightmap_sampler: sampler"));
    assert!(shader.contains("fn sample_lightmap(light: vec2<f32>) -> vec3<f32>"));
    assert!(shader.contains("light * (15.0 / 16.0) + vec2<f32>(0.5 / 16.0)"));
    assert!(shader.contains("vec2<f32>(15.5 / 16.0)"));
    assert!(shader.contains("textureSample(lightmap_texture, lightmap_sampler, uv).rgb"));
    assert!(shader.contains("var light_color = sample_lightmap(input.light)"));
    assert!(shader.contains("evaluate_dynamic_lights(input.fog_pos)"));
    assert!(!shader.contains("fn lightmap_brightness"));
    assert!(!shader.contains("camera.lightmap_factors.y"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER.contains("rgb * light_color"));
}

#[test]
fn entity_textured_shader_applies_vanilla_level_diffuse_lighting() {
    // Vanilla entity.vsh calls minecraft_mix_light with the current Lighting.Entry
    // directions, MINECRAFT_LIGHT_POWER 0.6 and ambient 0.4.
    let shader = ENTITY_MODEL_TEXTURED_SHADER;
    assert!(shader.contains("@location(5) normal: vec3<f32>"));
    assert!(shader.contains("minecraft_light0: vec4<f32>"));
    assert!(shader.contains("minecraft_light1: vec4<f32>"));
    assert!(shader.contains("let light0 = normalize(camera.minecraft_light0.xyz)"));
    assert!(shader.contains("let light1 = normalize(camera.minecraft_light1.xyz)"));
    assert!(shader.contains("(light_value.x + light_value.y) * 0.6 + 0.4"));
    assert!(ENTITY_MODEL_TEXTURED_SHADER
        .contains("texel.rgb * per_face_diffuse_light(input.normal, front_facing)"));
}

#[test]
fn entity_diffuse_formula_matches_vanilla_numeric_samples_and_back_faces() {
    let default = CameraUniform::identity().shader_lighting();
    let nether = CameraUniform::identity()
        .with_lightmap_environment(LightmapEnvironment {
            level_lighting: LevelLighting::Nether,
            ..LightmapEnvironment::default()
        })
        .shader_lighting();

    assert_close(vanilla_diffuse(default, Vec3::Y), 1.0);
    assert_close(vanilla_diffuse(default, Vec3::NEG_Y), 0.4);
    assert_close(vanilla_diffuse(default, Vec3::X), 0.49701425);
    assert_close(vanilla_diffuse(nether, Vec3::Y), 0.8850713);
    assert_close(vanilla_diffuse(nether, Vec3::NEG_Y), 0.8850713);

    let face_normal = Vec3::new(0.2, 0.8, 0.5).normalize();
    assert_close(vanilla_diffuse(default, face_normal), 1.0);
    assert_close(vanilla_diffuse(default, -face_normal), 0.4);
}

fn vanilla_diffuse(lights: ([f32; 3], [f32; 3]), normal: Vec3) -> f32 {
    let light0 = Vec3::from_array(lights.0).normalize();
    let light1 = Vec3::from_array(lights.1).normalize();
    ((light0.dot(normal).max(0.0) + light1.dot(normal).max(0.0)) * 0.6 + 0.4).min(1.0)
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1.0e-6,
        "actual {actual}, expected {expected}"
    );
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
fn banner_pattern_shader_and_pipeline_state_match_vanilla_26_1() {
    // `RenderPipelines.BANNER_PATTERN` inherits the entity snippet without
    // `ALPHA_CUTOUT` or `PER_FACE_LIGHTING`, defines `NO_OVERLAY`, enables
    // translucent blending, keeps default back-face culling and LESS_EQUAL,
    // and is the banner-specific entity pipeline whose depth write is false.
    assert!(ENTITY_MODEL_BANNER_PATTERN_SHADER
        .contains("textureSample(entity_texture_atlas, entity_sampler, input.uv) * input.tint"));
    assert!(ENTITY_MODEL_BANNER_PATTERN_SHADER.contains("diffuse_light(input.normal)"));
    assert!(ENTITY_MODEL_BANNER_PATTERN_SHADER.contains("sample_lightmap(input.light)"));
    assert!(ENTITY_MODEL_BANNER_PATTERN_SHADER.contains("return apply_fog("));
    for absent in [
        "discard",
        "input.overlay",
        "per_face_diffuse_light",
        "@builtin(front_facing)",
    ] {
        assert!(
            !ENTITY_MODEL_BANNER_PATTERN_SHADER.contains(absent),
            "bannerPattern shader must omit {absent}"
        );
    }
    assert_eq!(
        ENTITY_MODEL_BANNER_PATTERN_BLEND,
        Some(wgpu::BlendState::ALPHA_BLENDING)
    );
    const {
        assert!(!ENTITY_MODEL_BANNER_PATTERN_DEPTH_WRITE_ENABLED);
    }
    assert_eq!(
        ENTITY_MODEL_BANNER_PATTERN_DEPTH_COMPARE,
        wgpu::CompareFunction::LessEqual
    );
    assert_eq!(
        ENTITY_MODEL_BANNER_PATTERN_CULL_MODE,
        Some(wgpu::Face::Back)
    );
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
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x3, 6 => Float32x2, 7 => Float32x2, 8 => Float32x4]
    );
    // Eyes stay emissive: the lightmap shade must not dim them.
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("lightmap_brightness"));
    assert!(!ENTITY_MODEL_EYES_SHADER.contains("not_gamma"));
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

/// Every base-pass vertex of the instances' textured submission, merged in pass order, for pose
/// assertions that do not care which render type a part lands in.
fn entity_model_mesh(instances: &[EntityModelInstance]) -> EntityModelTexturedMesh {
    let atlas = EntityModelTextureAtlasLayout {
        width: 1,
        height: 1,
        entries: entity_model_texture_refs()
            .iter()
            .map(|texture| EntityModelTextureAtlasEntry {
                texture: *texture,
                uv: EntityModelUvRect {
                    min: [0.0, 0.0],
                    max: [1.0, 1.0],
                },
            })
            .collect(),
    };
    let meshes = entity_model_textured_meshes(instances, &atlas);
    let mut merged = EntityModelTexturedMesh::new();
    for pass in [
        &meshes.solid,
        &meshes.cutout,
        &meshes.cutout_cull,
        &meshes.solid_z_offset_forward,
        &meshes.cutout_z_offset,
        &meshes.translucent,
        &meshes.translucent_emissive,
        &meshes.item_entity_translucent,
        &meshes.item_entity_translucent_cull,
        &meshes.eyes,
    ] {
        let base = merged.vertices.len() as u32;
        merged.vertices.extend_from_slice(&pass.vertices);
        merged
            .indices
            .extend(pass.indices.iter().map(|index| index + base));
        merged.cutout_faces += pass.cutout_faces;
    }
    merged
}

/// The bind pose of a named part in a baked Vanilla layer.
/// Vanilla `KeyframeAnimations.degreeVec`.
fn degree_vec(x: f32, y: f32, z: f32) -> [f32; 3] {
    [x, y, z].map(|degrees| degrees * (std::f32::consts::PI / 180.0))
}

fn layer_pose(layer: ModelLayer, name: &str) -> PartPose {
    layer.bake().part_mut(name).unwrap().default_pose()
}

/// The vertices the named parts' subtrees emit, in render order.
fn parts_vertices<V: Copy>(vertices: &[V], root: &ModelPart, names: &[&str]) -> Vec<V> {
    names
        .iter()
        .flat_map(|name| {
            vertices[root.part_vertex_range(name).unwrap()]
                .iter()
                .copied()
        })
        .collect()
}

fn mesh_extents(mesh: &EntityModelTexturedMesh) -> ([f32; 3], [f32; 3]) {
    textured_mesh_extents(mesh)
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

fn assert_texture_atlas_layout_matches_refs(
    layout: &EntityModelTextureAtlasLayout,
    textures: &[EntityModelTextureRef],
) {
    let expected_width = textures
        .iter()
        .map(|texture| texture.size[0])
        .max()
        .unwrap_or_default();
    let expected_height = textures.iter().map(|texture| texture.size[1]).sum();
    assert_eq!(
        (layout.width, layout.height),
        (expected_width, expected_height)
    );
    assert_eq!(layout.entries.len(), textures.len());

    let mut y = 0;
    for (entry, texture) in layout.entries.iter().zip(textures) {
        assert_eq!(&entry.texture, texture);
        assert_close2(entry.uv.min, [0.0, y as f32 / expected_height as f32]);
        y += texture.size[1];
        assert_close2(
            entry.uv.max,
            [
                texture.size[0] as f32 / expected_width as f32,
                y as f32 / expected_height as f32,
            ],
        );
    }
}

fn texture_atlas_entry_y(textures: &[EntityModelTextureRef], index: usize) -> u32 {
    textures[..index]
        .iter()
        .map(|texture| texture.size[1])
        .sum()
}

fn spider_texture_images() -> Vec<EntityModelTextureImage> {
    [
        SPIDER_TEXTURE_REF,
        CAVE_SPIDER_TEXTURE_REF,
        SPIDER_EYES_TEXTURE_REF,
    ]
    .iter()
    .enumerate()
    .map(|(index, texture)| {
        let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
        EntityModelTextureImage::new(*texture, vec![index as u8; len])
    })
    .collect()
}

#[test]
fn invisible_living_entities_keep_vanilla_ungated_layers() {
    let textures = [
        SPIDER_TEXTURE_REF,
        SPIDER_EYES_TEXTURE_REF,
        ENDERMAN_TEXTURE_REF,
        ENDERMAN_EYES_TEXTURE_REF,
        PHANTOM_TEXTURE_REF,
        PHANTOM_EYES_TEXTURE_REF,
        CREAKING_TEXTURE_REF,
        CREAKING_EYES_TEXTURE_REF,
        BREEZE_TEXTURE_REF,
        BREEZE_WIND_TEXTURE_REF,
        BREEZE_EYES_TEXTURE_REF,
        CREEPER_TEXTURE_REF,
        CREEPER_ARMOR_TEXTURE_REF,
        WITHER_TEXTURE_REF,
        WITHER_ARMOR_TEXTURE_REF,
        GUARDIAN_TEXTURE_REF,
        GUARDIAN_BEAM_TEXTURE_REF,
        HORSE_WHITE_TEXTURE_REF,
        HORSE_SADDLE_TEXTURE_REF,
        HORSE_BODY_IRON_TEXTURE_REF,
        PIG_SADDLE_TEXTURE_REF,
        PLAYER_WIDE_STEVE_TEXTURE_REF,
        TRIDENT_RIPTIDE_TEXTURE_REF,
        COPPER_GOLEM_TEXTURE_REF,
        COPPER_GOLEM_EYES_TEXTURE_REF,
    ];
    let images: Vec<_> = textures
        .iter()
        .map(|texture| {
            let len = usize::try_from(texture.size[0] * texture.size[1] * 4).unwrap();
            EntityModelTextureImage::new(*texture, vec![255; len])
        })
        .collect();
    let (atlas, _) = build_entity_model_texture_atlas(&images).unwrap();
    let equipment = test_static_equipment_catalog();

    let invisible_state = |mut state: EntityRenderState| {
        state.invisible = true;
        state.invisible_to_player = true;
        state
    };
    let build = |mut inst: EntityModelInstance, state: EntityRenderState| {
        inst.render_state = state;
        entity_model_textured_meshes_with_equipment_for_camera(
            &[inst],
            &atlas,
            &[],
            &equipment,
            None,
            None,
            None,
            None,
        )
    };

    // Completely invisible spider still submits EyesLayer.
    let spider = EntityModelInstance::new(1, EntityModelKind::Spider, [0.0, 64.0, 0.0], 0.0);
    let meshes = build(spider, invisible_state(EntityRenderState::default()));
    assert!(meshes.cutout.indices.is_empty());
    assert!(!meshes.eyes.indices.is_empty());

    // Completely invisible creaking with glowing eyes keeps its `alwaysVisible = true` eyes layer.
    let creaking = EntityModelInstance::new(
        2,
        EntityModelKind::Creaking { eyes_glowing: true },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let meshes = build(creaking, invisible_state(EntityRenderState::default()));
    assert!(meshes.cutout.indices.is_empty());
    assert!(!meshes.eyes.indices.is_empty());

    // Completely invisible copper golem hides its `alwaysVisible = false` eyes layer.
    let copper_golem = EntityModelInstance::new(
        3,
        EntityModelKind::CopperGolem {
            weathering: CopperGolemWeathering::Unaffected,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let meshes = build(copper_golem, invisible_state(EntityRenderState::default()));
    assert!(meshes.cutout.indices.is_empty());
    assert!(meshes.eyes.indices.is_empty());

    // Completely invisible breeze keeps both BreezeWindLayer and BreezeEyesLayer.
    let breeze = EntityModelInstance::new(4, EntityModelKind::Breeze, [0.0, 64.0, 0.0], 0.0);
    let meshes = build(breeze, invisible_state(EntityRenderState::default()));
    assert!(!meshes.scroll.indices.is_empty());
    assert!(!meshes.translucent_emissive.indices.is_empty());

    // Completely invisible charged creeper keeps CreeperPowerLayer (`EnergySwirlLayer`).
    let mut creeper_state = invisible_state(EntityRenderState::default());
    creeper_state.kind.creeper_mut().powered = true;
    let creeper = EntityModelInstance::new(5, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0);
    let meshes = build(creeper, creeper_state);
    assert!(meshes.cutout.indices.is_empty());
    assert!(!meshes.scroll_additive.indices.is_empty());

    // Completely invisible saddled horse keeps both armor and saddle (`SimpleEquipmentLayer`).
    let mut horse_state = invisible_state(EntityRenderState::default());
    horse_state.kind.equine_mut().body_equipment_asset = Some(test_iron_equipment_asset_id());
    horse_state.kind.equine_mut().saddle_equipment_asset = Some(test_saddle_equipment_asset_id());
    let horse = EntityModelInstance::new(
        6,
        EntityModelKind::Horse {
            variant: HorseColorVariant::White,
            baby: false,
            markings: HorseMarkings::None,
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let meshes = build(horse, horse_state);
    assert!(meshes.cutout.indices.is_empty());
    assert!(!meshes.armor_cutout.indices.is_empty());

    // Completely invisible riptide player keeps SpinAttackEffectLayer.
    let mut player_state = invisible_state(EntityRenderState::default());
    player_state.auto_spin_age_ticks = Some(4.0);
    let player = EntityModelInstance::new(
        7,
        EntityModelKind::Player {
            skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
            parts: PlayerModelPartVisibility::from_vanilla_mask(0x7f),
        },
        [0.0, 64.0, 0.0],
        0.0,
    );
    let meshes = build(player, player_state);
    assert!(!meshes.cutout.indices.is_empty());
}
