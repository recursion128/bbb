use super::catalog::EntityDynamicPlayerTextureAtlasEntry;
use super::dispatch::{
    dispatch_invisible_living_ungated_layers, dispatch_uniform_entity_model,
    dispatch_vanilla_entity_layers, TexturedSink,
};
use super::equipment::{
    equipment_layer_tint, EntityEquipmentLayerType, ResolvedEntityEquipmentAssetCatalog,
    ResolvedEntityEquipmentAssetLayer, ResolvedEntityEquipmentAssetLayerTexture,
};
use super::held_item::custom_head_skull_transform;
use super::model::ModelPart;
#[cfg(test)]
use super::model_layers::PLAYER_WIDE_STEVE_TEXTURE_REF;
use super::transforms::{
    boat_model_root_transform, creeper_model_root_transform, ender_dragon_model_root_transform,
    trident_model_root_transform, wither_model_root_transform, zombie_variant_root_transform,
};
use super::{
    catalog::{
        ArrowModelTexture, CamelModelFamily, DonkeyModelFamily, EndPortalModelFace,
        EndPortalModelFaces, EndPortalModelKind, EntityArmorTrim, EntityCustomHeadSkull,
        EntityDynamicPlayerSkin, EntityDynamicPlayerSkinAtlasEntry,
        EntityDynamicPlayerSkinAtlasLayout, EntityDynamicPlayerSkinStatus,
        EntityDynamicPlayerTexture, EntityDynamicPlayerTextureAtlasLayout, EntityDynamicTexture,
        EntityModelKind, EntityModelOwnedTextureAtlasEntry, EntityModelTextureAtlasEntry,
        EntityModelTextureAtlasLayout, EntityModelTextureRef, EntityModelUvRect, EntityPlayerSkin,
        FirstPersonPlayerArm, HorseColorVariant, HorseMarkings, LeashRopeRenderState,
        ParrotModelVariant, PiglinModelFamily, PlayerModelPartVisibility, SkeletonModelFamily,
        UndeadHorseModelFamily, VillagerModelData, ZombieVariantModelFamily,
        PLAYER_MODEL_PARTS_ALL_VISIBLE,
    },
    entity_model_root_transform,
    geometry::{
        append_dissolve_textured_mesh, append_scrolled_textured_mesh, argb_to_tint,
        fill_entity_textured_light, fill_entity_textured_overlay, part_pose_transform,
        EntityModelDissolveMesh, EntityModelMesh, EntityModelPortalMesh, EntityModelPortalVertex,
        EntityModelScrollMesh, EntityModelScrollVertex, EntityModelTexturedMesh,
        EntityModelTexturedVertex, EntityModelVertex, PartPose, ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
        ENTITY_VERTEX_NO_OVERLAY,
    },
    instances::{BlockEntityCrumbling, EntityModelInstance},
    model_layers::{
        apply_player_part_visibility, armor_stand_model, breeze_model, camel_saddle_model,
        creeper_model, custom_head_dragon_skull_model, custom_head_piglin_skull_model,
        default_player_skin_texture_ref, elytra_model, end_crystal_get_y, equine_model,
        giant_model, happy_ghast_harness_model, happy_ghast_model, llama_model, nautilus_model,
        parrot_model, pig_model, piglin_model, player_ears_model, player_model, player_model_layer,
        skeleton_clothing_model, skeleton_model, spin_attack_effect_model, strider_model,
        villager_model, wither_model, wolf_armor_model, zombie_model, zombie_variant_model,
        EquineModelClass, HumanoidArmorModelLayerSet, HumanoidArmorSlot, ARROW_TEXTURE_REF,
        BEE_STINGER_TEXTURE_REF, CREEPER_TEXTURE_REF, DRAGON_FIREBALL_TEXTURE_REF,
        ENCHANTED_GLINT_ARMOR_TEXTURE_REF, ENDER_DRAGON_EXPLODING_TEXTURE_REF,
        ENDER_DRAGON_TEXTURE_REF, END_PORTAL_TEXTURE_REF, END_SKY_TEXTURE_REF,
        EXPERIENCE_ORB_TEXTURE_REF, FISHING_HOOK_TEXTURE_REF, HAPPY_GHAST_ROPES_TEXTURE_REF,
        HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND, HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND_SMALL,
        HUMANOID_ARMOR_MODEL_LAYERS_BOGGED, HUMANOID_ARMOR_MODEL_LAYERS_DROWNED,
        HUMANOID_ARMOR_MODEL_LAYERS_DROWNED_BABY, HUMANOID_ARMOR_MODEL_LAYERS_GIANT,
        HUMANOID_ARMOR_MODEL_LAYERS_HUSK, HUMANOID_ARMOR_MODEL_LAYERS_HUSK_BABY,
        HUMANOID_ARMOR_MODEL_LAYERS_PARCHED, HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN,
        HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BABY, HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE,
        HUMANOID_ARMOR_MODEL_LAYERS_PLAYER, HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM,
        HUMANOID_ARMOR_MODEL_LAYERS_SKELETON, HUMANOID_ARMOR_MODEL_LAYERS_STRAY,
        HUMANOID_ARMOR_MODEL_LAYERS_WITHER_SKELETON, HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY, HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER_BABY,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN,
        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN_BABY, PIGLIN_TEXTURE_REF,
        PLAYER_PROFILE_CAPE_TEXTURE_REF, PLAYER_PROFILE_ELYTRA_TEXTURE_REF,
        PLAYER_STUCK_BODY_PARTS, SKELETON_TEXTURE_REF, WITHER_SKELETON_TEXTURE_REF,
        ZOMBIE_TEXTURE_REF,
    },
    player_model_root_transform,
};
use crate::entity_models::model_layer::ModelLayer;
use crate::{
    block_destroy::{BlockDestroyOverlayMesh, BlockDestroyVertex},
    particles::{camera_billboard_orientation, ParticleItemPickupProjectileKind},
    sheeted_decal::sheeted_decal_uv,
};
use glam::{Mat4, Quat, Vec3};
use std::cmp::Ordering;

#[cfg(test)]
pub(super) use layers::player_textured_layer_passes;
pub(super) use layers::shulker_bullet_textured_layer_passes;
#[cfg(test)]
pub(super) use layers::warden_pulsating_spots_alpha;
pub(crate) use layers::EntityModelLayerRenderType;
#[cfg(test)]
pub(in crate::entity_models) use layers::EntityModelLayerVisibility;
pub(super) use layers::{
    armadillo_textured_layer_passes, arrow_textured_layer_passes, axolotl_textured_layer_passes,
    banner_textured_layer_passes, beacon_beam_layer_passes, bed_textured_layer_passes,
    bell_textured_layer_passes, blaze_textured_layer_passes, boat_textured_layer_passes,
    book_textured_layer_passes, breeze_textured_layer_passes, camel_textured_layer_passes,
    chest_textured_layer_passes, chicken_textured_layer_passes, conduit_textured_layer_passes,
    copper_golem_statue_block_layer_passes, copper_golem_textured_layer_passes,
    cow_textured_layer_passes, creaking_textured_layer_passes, creeper_textured_layer_passes,
    custom_head_skull_layer_pass, decorated_pot_textured_layer_passes,
    donkey_textured_layer_passes, drowned_textured_layer_passes, end_crystal_textured_layer_passes,
    end_gateway_beam_layer_passes, ender_dragon_textured_layer_passes,
    enderman_textured_layer_passes, endermite_textured_layer_passes, equipment_layer_pass,
    evoker_fangs_textured_layer_passes, feline_textured_layer_passes, fox_textured_layer_passes,
    frog_textured_layer_passes, ghast_textured_layer_passes, goat_textured_layer_passes,
    guardian_textured_layer_passes, happy_ghast_textured_layer_passes,
    hoglin_textured_layer_passes, horse_textured_layer_passes, husk_textured_layer_passes,
    illager_textured_layer_passes, iron_golem_textured_layer_passes,
    leash_knot_textured_layer_passes, llama_spit_textured_layer_passes,
    llama_textured_layer_passes, magma_cube_textured_layer_passes, minecart_textured_layer_passes,
    mooshroom_textured_layer_passes, nautilus_textured_layer_passes, panda_textured_layer_passes,
    parrot_textured_layer_passes, phantom_textured_layer_passes, pig_textured_layer_passes,
    piglin_textured_layer_passes, player_cape_layer_pass,
    player_extra_ears_layer_pass_with_texture, player_parrot_on_shoulder_layer_pass,
    player_spin_attack_effect_layer_pass, player_textured_layer_passes_with_texture,
    polar_bear_textured_layer_passes, rabbit_textured_layer_passes, ravager_textured_layer_passes,
    salmon_textured_layer_passes, sheep_textured_layer_passes, shulker_box_textured_layer_passes,
    shulker_textured_layer_passes, sign_textured_layer_passes, silverfish_textured_layer_passes,
    skeleton_textured_layer_passes, slime_textured_layer_passes, sniffer_textured_layer_passes,
    snow_golem_textured_layer_passes, spider_textured_layer_passes, squid_textured_layer_passes,
    tadpole_textured_layer_passes, trident_textured_layer_passes,
    tropical_fish_textured_layer_passes, undead_horse_textured_layer_passes,
    villager_data_textured_layer_passes, villager_textured_layer_passes, villager_type_hat_visible,
    wandering_trader_textured_layer_passes, warden_textured_layer_passes,
    wind_charge_textured_layer_passes, witch_textured_layer_passes,
    wither_skull_textured_layer_passes, wither_textured_layer_passes, wolf_textured_layer_passes,
    zombie_nautilus_textured_layer_passes, zombie_textured_layer_passes,
    zombie_villager_data_textured_layer_passes, zombie_villager_textured_layer_passes,
    EntityModelLayerKind, EntityModelLayerPass, EntityModelLayerRenderBucket,
};
pub(in crate::entity_models) use layers::{
    bed_model_layer, boat_model_layer, chicken_model_layer, cow_model_layer, donkey_model_layer,
    feline_collar_model_layer, feline_model_layer, fox_model_layer, goat_model_layer,
    horse_model_layer, mooshroom_model_layer, panda_model_layer, pig_model_layer,
    rabbit_model_layer, skeleton_model_layer, undead_horse_model_layer,
};

mod cache;
mod equipment_layers;
mod layers;
mod meshes;
pub(in crate::entity_models) use cache::resort_ranges;
pub(crate) use cache::EntityExtractionCache;
mod special_models;
pub(in crate::entity_models) use equipment_layers::*;
pub(crate) use meshes::*;
#[cfg(test)]
pub(in crate::entity_models) use special_models::dynamic_player_texture_test_meshes;
use special_models::emit_leash_rope_custom_submission;
pub(crate) use special_models::fishing_hook_line_segments;
pub(in crate::entity_models) use special_models::{
    elder_guardian_particle_textured_meshes, entity_model_water_mask_mesh,
    experience_orb_pickup_particle_textured_mesh, first_person_player_arm_textured_meshes,
    projectile_pickup_particle_textured_meshes, render_beacon_beam_model,
    render_breeze_wind_scroll_model, render_charged_creeper_energy_swirl, render_dragon_fireball,
    render_end_crystal_beam, render_end_portal_block_model, render_ender_dragon_beam,
    render_ender_dragon_death_rays, render_experience_orb, render_fishing_hook,
    render_guardian_beam, render_no_overlay_scrolled_textured_layers,
    render_trident_foil_submission, render_wither_energy_swirl,
};
pub use special_models::{
    ElderGuardianParticleRenderInstance, ExperienceOrbPickupParticleRenderInstance,
    ProjectilePickupParticleRenderInstance,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EntityModelRenderSubmission {
    pub(super) render_type: EntityModelLayerRenderType,
    pub(super) texture: EntityModelTextureRef,
    /// Index into the selected shared entity atlas's owned-texture table.
    /// `None` means `texture` is a static catalog reference.
    pub(super) owned_texture: Option<u32>,
    pub(super) dissolve_texture: Option<EntityModelTextureRef>,
    pub(super) tint: [f32; 4],
    pub(super) transform: Mat4,
    pub(super) light: [f32; 2],
    pub(super) overlay: [f32; 2],
    pub(super) outline_color: u32,
    pub(super) order: i32,
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(super) fn entity_model_textured_meshes_with_equipment_for_camera(
    instances: &[EntityModelInstance],
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    sort_camera_position: Option<[f32; 3]>,
    camera_pose: Option<crate::CameraPose>,
) -> EntityModelTexturedMeshes {
    entity_model_textured_meshes_with_equipment_for_camera_orientation(
        instances,
        atlas,
        owned_entries,
        equipment_assets,
        dynamic_player_skin_atlas,
        dynamic_player_texture_atlas,
        sort_camera_position,
        camera_pose.map(camera_billboard_orientation),
    )
}

const PAINTING_DYNAMIC_TEXTURE_FALLBACK_REF: EntityModelTextureRef = EntityModelTextureRef {
    path: "textures/painting/back.png",
    size: [16, 16],
};

/// Emits one tile of vanilla `PaintingRenderer.renderPainting` custom geometry. The scene expands
/// the complete painting into independently lit tile instances; this helper retains the exact
/// front partitioning, full back sprite, and boundary-only thin edges on the `paintings` atlas.
pub(in crate::entity_models) fn render_painting_tile(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::PaintingTile {
        width,
        height,
        segment_x,
        segment_y,
        facing,
        front,
        back,
    } = instance.kind
    else {
        return;
    };
    if width <= 0
        || height <= 0
        || !(0..width).contains(&segment_x)
        || !(0..height).contains(&segment_y)
        || matches!(
            facing,
            super::catalog::EntityAttachmentFace::Down | super::catalog::EntityAttachmentFace::Up
        )
        || meshes.current_outline_only
    {
        return;
    }
    let Some(front_entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, front) else {
        return;
    };
    let Some(back_entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, back) else {
        return;
    };
    let yaw: f32 = match facing {
        super::catalog::EntityAttachmentFace::North => 0.0,
        super::catalog::EntityAttachmentFace::South => 180.0,
        super::catalog::EntityAttachmentFace::West => 90.0,
        super::catalog::EntityAttachmentFace::East => -90.0,
        super::catalog::EntityAttachmentFace::Down | super::catalog::EntityAttachmentFace::Up => {
            unreachable!("vertical painting facings were rejected")
        }
    };
    let transform = Mat4::from_translation(Vec3::from_array(instance.position))
        * Mat4::from_rotation_y(yaw.to_radians());
    let (render_type, tint) = if meshes.current_force_transparent {
        (
            EntityModelLayerRenderType::EntityTranslucentCullItemTarget,
            [1.0, 1.0, 1.0, 38.0 / 255.0],
        )
    } else {
        (
            EntityModelLayerRenderType::EntitySolidZOffsetForward,
            [1.0; 4],
        )
    };
    let submit = meshes.submission(
        render_type,
        PAINTING_DYNAMIC_TEXTURE_FALLBACK_REF,
        tint,
        transform,
        0,
    );
    render_textured_dynamic_player_texture_submission(
        meshes,
        submit,
        front_entry,
        |mesh, front_entry| {
            append_painting_tile_geometry(
                mesh,
                transform,
                width,
                height,
                segment_x,
                segment_y,
                front_entry.uv,
                back_entry.uv,
            );
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn append_painting_tile_geometry(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    width: i32,
    height: i32,
    segment_x: i32,
    segment_y: i32,
    front: EntityModelUvRect,
    back: EntityModelUvRect,
) {
    let offset_x = -(width as f32) / 2.0;
    let offset_y = -(height as f32) / 2.0;
    let x0 = offset_x + (segment_x + 1) as f32;
    let x1 = offset_x + segment_x as f32;
    let y0 = offset_y + (segment_y + 1) as f32;
    let y1 = offset_y + segment_y as f32;
    let edge = 0.03125;
    let front_u0 = (width - segment_x) as f32 / width as f32;
    let front_u1 = (width - (segment_x + 1)) as f32 / width as f32;
    let front_v0 = (height - segment_y) as f32 / height as f32;
    let front_v1 = (height - (segment_y + 1)) as f32 / height as f32;
    append_painting_quad(
        mesh,
        transform,
        [
            ([x0, y1, -edge], painting_uv(front, front_u1, front_v0)),
            ([x1, y1, -edge], painting_uv(front, front_u0, front_v0)),
            ([x1, y0, -edge], painting_uv(front, front_u0, front_v1)),
            ([x0, y0, -edge], painting_uv(front, front_u1, front_v1)),
        ],
        [0.0, 0.0, -1.0],
    );
    append_painting_quad(
        mesh,
        transform,
        [
            ([x0, y0, edge], painting_uv(back, 1.0, 0.0)),
            ([x1, y0, edge], painting_uv(back, 0.0, 0.0)),
            ([x1, y1, edge], painting_uv(back, 0.0, 1.0)),
            ([x0, y1, edge], painting_uv(back, 1.0, 1.0)),
        ],
        [0.0, 0.0, 1.0],
    );
    if segment_y == height - 1 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x0, y0, -edge], painting_uv(back, 0.0, 0.0)),
                ([x1, y0, -edge], painting_uv(back, 1.0, 0.0)),
                ([x1, y0, edge], painting_uv(back, 1.0, 1.0 / 16.0)),
                ([x0, y0, edge], painting_uv(back, 0.0, 1.0 / 16.0)),
            ],
            [0.0, 1.0, 0.0],
        );
    }
    if segment_y == 0 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x0, y1, edge], painting_uv(back, 0.0, 0.0)),
                ([x1, y1, edge], painting_uv(back, 1.0, 0.0)),
                ([x1, y1, -edge], painting_uv(back, 1.0, 1.0 / 16.0)),
                ([x0, y1, -edge], painting_uv(back, 0.0, 1.0 / 16.0)),
            ],
            [0.0, -1.0, 0.0],
        );
    }
    if segment_x == width - 1 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x0, y0, edge], painting_uv(back, 1.0 / 16.0, 0.0)),
                ([x0, y1, edge], painting_uv(back, 1.0 / 16.0, 1.0)),
                ([x0, y1, -edge], painting_uv(back, 0.0, 1.0)),
                ([x0, y0, -edge], painting_uv(back, 0.0, 0.0)),
            ],
            [-1.0, 0.0, 0.0],
        );
    }
    if segment_x == 0 {
        append_painting_quad(
            mesh,
            transform,
            [
                ([x1, y0, -edge], painting_uv(back, 1.0 / 16.0, 0.0)),
                ([x1, y1, -edge], painting_uv(back, 1.0 / 16.0, 1.0)),
                ([x1, y1, edge], painting_uv(back, 0.0, 1.0)),
                ([x1, y0, edge], painting_uv(back, 0.0, 0.0)),
            ],
            [1.0, 0.0, 0.0],
        );
    }
}

fn painting_uv(rect: EntityModelUvRect, u: f32, v: f32) -> [f32; 2] {
    [
        rect.min[0] + u * (rect.max[0] - rect.min[0]),
        rect.min[1] + v * (rect.max[1] - rect.min[1]),
    ]
}

fn append_painting_quad(
    mesh: &mut EntityModelTexturedMesh,
    transform: Mat4,
    vertices: [([f32; 3], [f32; 2]); 4],
    normal: [f32; 3],
) {
    let base = u32::try_from(mesh.vertex_count()).expect("painting vertex count fits in u32");
    let normal = transform
        .inverse()
        .transpose()
        .transform_vector3(Vec3::from_array(normal))
        .normalize_or_zero()
        .to_array();
    let quad = vertices.map(|(position, uv)| EntityModelTexturedVertex {
        position: transform
            .transform_point3(Vec3::from_array(position))
            .to_array(),
        uv,
        tint: [1.0; 4],
        light: ENTITY_VERTEX_FULL_BRIGHT_LIGHT,
        overlay: ENTITY_VERTEX_NO_OVERLAY,
        normal,
        standalone_local_uv: [0.0; 2],
        standalone_pixel_origin: [0.0; 2],
        standalone_size_flags: [0.0; 4],
    });
    let [a, b, c, _] = quad.map(|vertex| Vec3::from_array(vertex.position));
    mesh.vertices.extend(quad);
    if (b - a).cross(c - a).dot(Vec3::from_array(normal)) >= 0.0 {
        mesh.indices
            .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    } else {
        // Vanilla emits quads and lets the backend expand them. Normalize the triangle-list
        // expansion against the submitted normal so the dedicated entity-solid cull state keeps
        // every front/back/edge face visible on wgpu.
        mesh.indices
            .extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    }
    mesh.cutout_faces += 1;
}

/// Vanilla `EntityRenderer.submit` submits every extracted `LeashState` through
/// `SubmitNodeCollector.submitLeash` before any model or layer work, with no invisibility or glow
/// gate, so the rope emits for every instance that carries leash states.
pub(in crate::entity_models) fn render_leash_ropes(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
) {
    let Some(leash_ropes) = instance.render_state.leash_ropes else {
        return;
    };
    for rope in leash_ropes.ropes() {
        emit_leash_rope_custom_submission(meshes, instance.position, *rope);
    }
}

impl TexturedSink<'_> {
    /// Poses the host humanoid, then drapes the selected armor layers over its limb poses.
    pub(in crate::entity_models) fn worn_humanoid_armor(&mut self, instance: &EntityModelInstance) {
        let render_state = &instance.render_state;
        let no_asset = render_state.head_equipment_asset.is_none()
            && render_state.chest_equipment_asset.is_none()
            && render_state.legs_equipment_asset.is_none()
            && render_state.feet_equipment_asset.is_none();
        if no_asset {
            return;
        }
        match instance.kind {
            EntityModelKind::ArmorStand { .. } => emit_worn_armor_stand_armor(
                self.meshes,
                instance,
                self.atlas,
                self.owned_entries,
                self.equipment_assets,
                self.dynamic_player_texture_atlas,
            ),
            EntityModelKind::Zombie { baby } => self.humanoid_armor_model(
                &zombie_model(baby, instance),
                instance,
                entity_model_root_transform(instance),
                baby,
                if baby {
                    HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_BABY
                } else {
                    HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE
                },
            ),
            EntityModelKind::Giant => self.humanoid_armor_model(
                &giant_model(instance),
                instance,
                entity_model_root_transform(instance),
                false,
                HUMANOID_ARMOR_MODEL_LAYERS_GIANT,
            ),
            EntityModelKind::ZombieVariant { family, baby } => {
                let armor_model_layers = match (family, baby) {
                    (ZombieVariantModelFamily::Husk, false) => HUMANOID_ARMOR_MODEL_LAYERS_HUSK,
                    (ZombieVariantModelFamily::Husk, true) => HUMANOID_ARMOR_MODEL_LAYERS_HUSK_BABY,
                    (ZombieVariantModelFamily::Drowned, false) => {
                        HUMANOID_ARMOR_MODEL_LAYERS_DROWNED
                    }
                    (ZombieVariantModelFamily::Drowned, true) => {
                        HUMANOID_ARMOR_MODEL_LAYERS_DROWNED_BABY
                    }
                    (ZombieVariantModelFamily::ZombieVillager, false) => {
                        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER
                    }
                    (ZombieVariantModelFamily::ZombieVillager, true) => {
                        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIE_VILLAGER_BABY
                    }
                };
                self.humanoid_armor_model(
                    &zombie_variant_model(family, baby, instance),
                    instance,
                    zombie_variant_root_transform(instance, family),
                    baby,
                    armor_model_layers,
                );
            }
            EntityModelKind::Skeleton => self.humanoid_armor_model(
                &skeleton_model(None, instance),
                instance,
                entity_model_root_transform(instance),
                false,
                HUMANOID_ARMOR_MODEL_LAYERS_SKELETON,
            ),
            EntityModelKind::SkeletonVariant { family } => {
                let armor_model_layers = match family {
                    SkeletonModelFamily::Stray => HUMANOID_ARMOR_MODEL_LAYERS_STRAY,
                    SkeletonModelFamily::Parched => HUMANOID_ARMOR_MODEL_LAYERS_PARCHED,
                    SkeletonModelFamily::WitherSkeleton => {
                        HUMANOID_ARMOR_MODEL_LAYERS_WITHER_SKELETON
                    }
                    SkeletonModelFamily::Bogged { .. } => HUMANOID_ARMOR_MODEL_LAYERS_BOGGED,
                };
                self.humanoid_armor_model(
                    &skeleton_model(Some(family), instance),
                    instance,
                    entity_model_root_transform(instance),
                    false,
                    armor_model_layers,
                );
            }
            EntityModelKind::Player { skin, .. } => {
                let slim = skin.is_slim();
                self.humanoid_armor_model(
                    &player_model(slim, instance),
                    instance,
                    player_model_root_transform(instance),
                    false,
                    if slim {
                        HUMANOID_ARMOR_MODEL_LAYERS_PLAYER_SLIM
                    } else {
                        HUMANOID_ARMOR_MODEL_LAYERS_PLAYER
                    },
                );
            }
            EntityModelKind::Piglin { family, baby } => {
                // Piglin brutes use adult armor even when the host carries the baby flag.
                let armor_baby = baby && family != PiglinModelFamily::PiglinBrute;
                let armor_model_layers = match (family, armor_baby) {
                    (PiglinModelFamily::Piglin, false) => HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN,
                    (PiglinModelFamily::Piglin, true) => HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BABY,
                    (PiglinModelFamily::PiglinBrute, _) => HUMANOID_ARMOR_MODEL_LAYERS_PIGLIN_BRUTE,
                    (PiglinModelFamily::ZombifiedPiglin, false) => {
                        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN
                    }
                    (PiglinModelFamily::ZombifiedPiglin, true) => {
                        HUMANOID_ARMOR_MODEL_LAYERS_ZOMBIFIED_PIGLIN_BABY
                    }
                };
                self.humanoid_armor_model(
                    &piglin_model(family, baby, instance),
                    instance,
                    entity_model_root_transform(instance),
                    armor_baby,
                    armor_model_layers,
                );
            }
            _ => {}
        }
    }

    fn humanoid_armor_model(
        &mut self,
        host: &ModelPart,
        instance: &EntityModelInstance,
        transform: Mat4,
        baby: bool,
        armor_model_layers: HumanoidArmorModelLayerSet,
    ) {
        emit_humanoid_armor(
            self.meshes,
            instance,
            host,
            transform,
            baby,
            armor_model_layers,
            self.atlas,
            self.owned_entries,
            self.equipment_assets,
            self.dynamic_player_texture_atlas,
        );
    }
}

/// Vanilla `HappyGhastRenderer` `SimpleEquipmentLayer(HAPPY_GHAST_BODY)`. Both ages have a harness
/// model; the base texture is followed by optional armor glint and trim submissions.
pub(in crate::entity_models) fn render_happy_ghast_harness_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::HappyGhast { baby } = instance.kind else {
        return;
    };

    let transform = entity_model_root_transform(instance);
    let model = happy_ghast_harness_model(baby, instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance
            .render_state
            .kind
            .happy_ghast()
            .body_equipment_asset,
        EntityEquipmentLayerType::HappyGhastBody,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.happy_ghast().harness_dye,
        instance.render_state.kind.happy_ghast().harness_foil,
        instance.render_state.kind.happy_ghast().harness_trim,
        true,
        0,
        atlas,
        owned_entries,
        dynamic_texture_atlas,
        None,
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    )
}

/// Vanilla `PigRenderer` `SimpleEquipmentLayer(PIG_SADDLE)`: when the saddle slot contains the
/// saddle item, render an adult `PigModel.createBodyLayer(CubeDeformation(0.5F))` over the base pig
/// with the `pig_saddle/saddle.png` equipment texture. The vanilla layer has no baby model, so baby
/// pigs skip it even if the slot is filled.
pub(in crate::entity_models) fn render_pig_saddle_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if instance
        .render_state
        .kind
        .pig()
        .saddle_equipment_asset
        .is_none()
    {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Pig { baby: false, .. }) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let model = pig_model(ModelLayer::PIG_SADDLE, instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.pig().saddle_equipment_asset,
        EntityEquipmentLayerType::PigSaddle,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.pig().saddle_dye,
        instance.render_state.kind.pig().saddle_foil,
        instance.render_state.kind.pig().saddle_trim,
        true,
        0,
        atlas,
        owned_entries,
        dynamic_texture_atlas,
        None,
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    );
}

/// Vanilla `HorseRenderer` / `UndeadHorseRenderer` `SimpleEquipmentLayer(HORSE_BODY)`: an adult horse
/// or zombie horse with a body armor item renders `HorseModel(ModelLayers.*_HORSE_ARMOR)`. The living
/// horse armor model inherits the 1.1 `livingHorseScale`; the zombie horse armor model is unscaled.
/// Vanilla supplies no baby model. Skeleton horses use the same renderer class but the vanilla
/// `CAN_WEAR_HORSE_ARMOR` tag excludes them, so the world projection never sets this layer for them.
/// The shared equipment renderer inserts foil after the first base layer and a runtime-atlas trim
/// after all base layers, before a later saddle equipment layer.
pub(in crate::entity_models) fn render_equine_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let (model_layer, order) = match instance.kind {
        EntityModelKind::Horse { baby: false, .. } => (ModelLayer::HORSE_ARMOR, 2),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false,
        } => (ModelLayer::UNDEAD_HORSE_ARMOR, 0),
        _ => return,
    };
    let transform = entity_model_root_transform(instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.equine().body_equipment_asset,
        EntityEquipmentLayerType::HorseBody,
    ) else {
        return;
    };
    let model = equine_model(model_layer, EquineModelClass::Adult, instance);
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.equine().body_armor_dye,
        instance.render_state.kind.equine().body_armor_foil,
        instance.render_state.kind.equine().body_armor_trim,
        true,
        order,
        atlas,
        owned_entries,
        dynamic_texture_atlas,
        None,
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    )
}

pub(in crate::entity_models) fn render_wings_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let Some(layers) = instance
        .render_state
        .chest_wings_equipment_asset
        .and_then(|asset_id| equipment_assets.asset(asset_id))
        .map(|asset| asset.layers(EntityEquipmentLayerType::Wings))
    else {
        return;
    };
    let Some((transform, baby)) = wings_layer_transform_and_baby(instance) else {
        return;
    };

    let model = elytra_model(baby, instance);
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.chest_armor_dye,
        instance.render_state.chest_wings_foil,
        instance.render_state.chest_wings_trim,
        true,
        0,
        atlas,
        owned_entries,
        dynamic_player_texture_atlas,
        player_wings_profile_texture_override(instance),
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    );
}
