use super::*;
use crate::entity_models::model_layers::{
    WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF, WOLF_ARMOR_CRACKINESS_LOW_TEXTURE_REF,
    WOLF_ARMOR_CRACKINESS_MEDIUM_TEXTURE_REF,
};
use crate::entity_models::WolfArmorCrackiness;
use bbb_registry_codec::legacy_random::LegacyRandomSource;

const MAX_PLAYER_STUCK_ITEMS_PER_KIND: usize = 64;

const OWNED_EQUIPMENT_TEXTURE_PROXY_PATH: &str = "bbb:owned_equipment_texture";

fn resolved_equipment_layer_texture(
    layer: ResolvedEntityEquipmentAssetLayer,
) -> (EntityModelTextureRef, Option<u32>) {
    match layer.texture {
        ResolvedEntityEquipmentAssetLayerTexture::Static(texture) => (texture, None),
        ResolvedEntityEquipmentAssetLayerTexture::Owned {
            index,
            logical_size,
        } => (
            EntityModelTextureRef {
                path: OWNED_EQUIPMENT_TEXTURE_PROXY_PATH,
                size: logical_size,
            },
            Some(index),
        ),
    }
}

pub(super) fn resolved_equipment_asset_layers(
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    asset_id: Option<bbb_render_types::EntityEquipmentAssetId>,
    layer_type: EntityEquipmentLayerType,
) -> Option<&[ResolvedEntityEquipmentAssetLayer]> {
    asset_id
        .and_then(|asset_id| equipment_assets.asset(asset_id))
        .map(|asset| asset.layers(layer_type))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_equipment_asset_layers(
    meshes: &mut EntityModelTexturedMeshes,
    layers: &[ResolvedEntityEquipmentAssetLayer],
    kind: EntityModelLayerKind,
    transform: Mat4,
    dye: Option<u32>,
    foil: bool,
    trim: Option<EntityArmorTrim>,
    trim_visible: bool,
    starting_order: i32,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
    player_texture_override: Option<EntityDynamicPlayerTexture>,
    mut emit: impl FnMut(&mut EntityModelTexturedMesh, EntityModelUvRect, [f32; 4]),
) {
    if layers.is_empty() {
        return;
    }
    let mut next_order = starting_order;
    let mut foil_pending = foil;
    let mut trim_base_texture = resolved_equipment_layer_texture(layers[0]).0;
    for layer in layers.iter().copied() {
        let Some(tint) = equipment_layer_tint(layer.dyeable, dye) else {
            continue;
        };
        let (equipment_texture, owned_texture) = resolved_equipment_layer_texture(layer);
        let profile_texture = layer
            .use_player_texture
            .then_some(player_texture_override)
            .flatten();
        let pass_texture = profile_texture
            .map(player_profile_wings_texture_ref)
            .unwrap_or(equipment_texture);
        trim_base_texture = pass_texture;
        let pass = equipment_layer_pass(
            kind,
            EntityModelLayerRenderType::ArmorCutoutNoCull,
            pass_texture,
            tint,
            next_order,
        );
        let mut submit = textured_layer_submission(meshes, pass, transform);
        if let Some(profile_texture) = profile_texture {
            if let Some(entry) =
                dynamic_player_texture_atlas_entry(dynamic_texture_atlas, profile_texture)
            {
                render_textured_dynamic_player_texture_submission(
                    meshes,
                    submit,
                    entry,
                    |mesh, entry| emit(mesh, entry.uv, submit.tint),
                );
            }
        } else {
            if let Some(owned_texture) = owned_texture {
                submit = submit.with_owned_texture(owned_texture);
            }
            render_textured_submission_with_owned(
                meshes,
                submit,
                atlas,
                owned_entries,
                |mesh, entry| emit(mesh, entry.uv, submit.tint),
            );
        }
        next_order += 1;

        if foil_pending {
            let pass = equipment_layer_pass(
                kind,
                EntityModelLayerRenderType::ArmorEntityGlint,
                ENCHANTED_GLINT_ARMOR_TEXTURE_REF,
                tint,
                next_order,
            );
            let submit = no_overlay_layer_submission(meshes, pass, transform);
            render_textured_submission(meshes, submit, atlas, |mesh, entry| {
                emit(mesh, entry.uv, submit.tint);
            });
            next_order += 1;

            foil_pending = false;
        }
    }

    let Some(trim) = trim.filter(|_| trim_visible) else {
        return;
    };
    let render_type = if trim.decal {
        EntityModelLayerRenderType::ArmorTrimDecalCutoutNoCull
    } else {
        EntityModelLayerRenderType::ArmorTrimCutoutNoCull
    };
    let pass = equipment_layer_pass(kind, render_type, trim_base_texture, [1.0; 4], next_order);
    let submit = no_overlay_layer_submission(meshes, pass, transform);
    if let Some(entry) = dynamic_texture_atlas_entry(dynamic_texture_atlas, trim.texture) {
        render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
            emit(mesh, entry.uv, submit.tint);
        });
    }
}

/// The `HumanoidArmorLayer` worn-armor overlay (vanilla `HumanoidArmorLayer.submit`): for each filled
/// equipment slot the inflated `HumanoidArmorModel` piece (helmet / chestplate / leggings / boots) is
/// draped on the host humanoid's posed limbs ([`ModelPart::copy_child_poses_from`] = vanilla
/// `copyPropertiesTo`) and drawn into the cutout pass with the catalog equipment-asset texture. The
/// pieces render in the vanilla order (chest, legs, feet, head). `transform` is the host entity's root
/// transform so the armor sits exactly on the body. Enchanted armor records the vanilla
/// `armorEntityGlint` submission at `order(2)` after that
/// slot's `order(1)` armor layer; adult humanoid trims follow at the next order and baby humanoid
/// layer types suppress them.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_humanoid_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    host_root: &ModelPart,
    transform: Mat4,
    baby: bool,
    armor_model_layers: HumanoidArmorModelLayerSet,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let render_state = &instance.render_state;
    let trim_visible = match instance.kind {
        EntityModelKind::Zombie { baby }
        | EntityModelKind::ZombieVariant { baby, .. }
        | EntityModelKind::Piglin { baby, .. } => !baby,
        _ => true,
    };
    for (slot, asset_id, dye, foil, trim) in [
        (
            HumanoidArmorSlot::Chest,
            render_state.chest_equipment_asset,
            render_state.chest_armor_dye,
            render_state.chest_armor_foil,
            render_state.chest_armor_trim,
        ),
        (
            HumanoidArmorSlot::Legs,
            render_state.legs_equipment_asset,
            render_state.legs_armor_dye,
            render_state.legs_armor_foil,
            render_state.legs_armor_trim,
        ),
        (
            HumanoidArmorSlot::Feet,
            render_state.feet_equipment_asset,
            render_state.feet_armor_dye,
            render_state.feet_armor_foil,
            render_state.feet_armor_trim,
        ),
        (
            HumanoidArmorSlot::Head,
            render_state.head_equipment_asset,
            render_state.head_armor_dye,
            render_state.head_armor_foil,
            render_state.head_armor_trim,
        ),
    ] {
        let mut tree = armor_model_layers.model_layer(slot).bake();
        // Vanilla runs the armor model's own `setupAnim`, so its bind pivots stay its own.
        let names = if baby {
            slot.baby_pose_part_names()
        } else {
            slot.part_names()
        };
        tree.copy_child_animation_from(host_root, names);
        let layer_type = if baby {
            EntityEquipmentLayerType::HumanoidBaby
        } else if slot == HumanoidArmorSlot::Legs {
            EntityEquipmentLayerType::HumanoidLeggings
        } else {
            EntityEquipmentLayerType::Humanoid
        };
        let Some(layers) = asset_id
            .and_then(|asset_id| equipment_assets.asset(asset_id))
            .map(|asset| asset.layers(layer_type))
        else {
            continue;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::NoOverlay,
            transform,
            dye,
            foil,
            trim,
            trim_visible,
            1,
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, uv, tint| {
                tree.render_textured(mesh, transform, uv, tint);
            },
        );
    }
}

pub(super) fn emit_worn_armor_stand_armor(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::ArmorStand {
        small,
        show_arms,
        show_base_plate,
        pose,
        ..
    } = instance.kind
    else {
        return;
    };
    let render_state = &instance.render_state;
    let no_asset = render_state.head_equipment_asset.is_none()
        && render_state.chest_equipment_asset.is_none()
        && render_state.legs_equipment_asset.is_none()
        && render_state.feet_equipment_asset.is_none();
    if no_asset {
        return;
    }
    let host = armor_stand_model(small, show_arms, show_base_plate, pose, instance);
    let transform = entity_model_root_transform(instance);
    let armor_model_layers = if small {
        HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND_SMALL
    } else {
        HUMANOID_ARMOR_MODEL_LAYERS_ARMOR_STAND
    };
    for (slot, asset_id, dye, foil, trim) in [
        (
            HumanoidArmorSlot::Chest,
            render_state.chest_equipment_asset,
            render_state.chest_armor_dye,
            render_state.chest_armor_foil,
            render_state.chest_armor_trim,
        ),
        (
            HumanoidArmorSlot::Legs,
            render_state.legs_equipment_asset,
            render_state.legs_armor_dye,
            render_state.legs_armor_foil,
            render_state.legs_armor_trim,
        ),
        (
            HumanoidArmorSlot::Feet,
            render_state.feet_equipment_asset,
            render_state.feet_armor_dye,
            render_state.feet_armor_foil,
            render_state.feet_armor_trim,
        ),
        (
            HumanoidArmorSlot::Head,
            render_state.head_equipment_asset,
            render_state.head_armor_dye,
            render_state.head_armor_foil,
            render_state.head_armor_trim,
        ),
    ] {
        let mut tree = armor_model_layers.model_layer(slot).bake();
        tree.copy_child_animation_from(&host, slot.part_names());
        let layer_type = if slot == HumanoidArmorSlot::Legs {
            EntityEquipmentLayerType::HumanoidLeggings
        } else {
            EntityEquipmentLayerType::Humanoid
        };
        let Some(layers) = asset_id
            .and_then(|asset_id| equipment_assets.asset(asset_id))
            .map(|asset| asset.layers(layer_type))
        else {
            continue;
        };
        emit_equipment_asset_layers(
            meshes,
            layers,
            EntityModelLayerKind::NoOverlay,
            transform,
            dye,
            foil,
            trim,
            true,
            1,
            atlas,
            owned_entries,
            dynamic_texture_atlas,
            None,
            |mesh, uv, tint| {
                tree.render_textured(mesh, transform, uv, tint);
            },
        );
    }
}

pub(in crate::entity_models) fn render_skeleton_clothing_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let EntityModelKind::SkeletonVariant { family } = instance.kind else {
        return;
    };
    let Some(pass) = skeleton_textured_layer_passes(Some(family))
        .into_iter()
        .find(|pass| matches!(pass.kind, EntityModelLayerKind::SkeletonClothing))
    else {
        return;
    };
    let transform = entity_model_root_transform(instance);
    let model = skeleton_clothing_model(Some(family), instance);
    render_textured_layers(meshes, &model, transform, [pass], atlas);
}

pub(in crate::entity_models) fn render_custom_head_skull_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let Some(skull) = instance.render_state.custom_head_skull else {
        return;
    };
    let Some(transform) = custom_head_skull_transform(instance) else {
        return;
    };
    render_custom_head_skull_layer_at_transform(
        meshes,
        instance,
        skull,
        transform,
        atlas,
        dynamic_player_skin_atlas,
    );
}

pub(in crate::entity_models) fn render_custom_head_skull_layer_at_transform(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    skull: EntityCustomHeadSkull,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    render_skull_model(
        meshes,
        instance,
        skull,
        transform,
        atlas,
        dynamic_player_skin_atlas,
    );
}

pub(in crate::entity_models) fn render_skull_model(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    skull: EntityCustomHeadSkull,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let pass = custom_head_skull_layer_pass(skull, custom_head_skull_texture_ref(skull));
    let model_layer = match skull {
        EntityCustomHeadSkull::Dragon => {
            let model = custom_head_dragon_skull_model(instance);
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            return;
        }
        EntityCustomHeadSkull::Piglin => {
            let model = custom_head_piglin_skull_model(instance);
            render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
            return;
        }
        EntityCustomHeadSkull::Skeleton => ModelLayer::SKELETON_SKULL,
        EntityCustomHeadSkull::WitherSkeleton => ModelLayer::WITHER_SKELETON_SKULL,
        EntityCustomHeadSkull::Player(_) => ModelLayer::PLAYER_HEAD,
        EntityCustomHeadSkull::Zombie => ModelLayer::ZOMBIE_HEAD,
        EntityCustomHeadSkull::Creeper => ModelLayer::CREEPER_HEAD,
    };
    let model = model_layer.bake();
    if let Some(dynamic_player_skin) = custom_head_dynamic_player_skin(skull) {
        render_textured_no_overlay_layer_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            pass,
            dynamic_player_skin,
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_no_overlay_layer_pass(meshes, &model, transform, pass, atlas);
}

fn custom_head_dynamic_player_skin(
    skull: EntityCustomHeadSkull,
) -> Option<EntityDynamicPlayerSkin> {
    match skull {
        EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(skin)) => Some(skin),
        _ => None,
    }
}

fn custom_head_skull_texture_ref(skull: EntityCustomHeadSkull) -> EntityModelTextureRef {
    match skull {
        EntityCustomHeadSkull::Skeleton => SKELETON_TEXTURE_REF,
        EntityCustomHeadSkull::WitherSkeleton => WITHER_SKELETON_TEXTURE_REF,
        EntityCustomHeadSkull::Player(skin) => default_player_skin_texture_ref(skin.fallback()),
        EntityCustomHeadSkull::Zombie => ZOMBIE_TEXTURE_REF,
        EntityCustomHeadSkull::Creeper => CREEPER_TEXTURE_REF,
        EntityCustomHeadSkull::Dragon => ENDER_DRAGON_TEXTURE_REF,
        EntityCustomHeadSkull::Piglin => PIGLIN_TEXTURE_REF,
    }
}

/// Vanilla `HappyGhastRenderer` `RopesLayer`: after the harness equipment layer, render a second
/// happy-ghast model inflated by `CubeDeformation(0.2F)`. The synchronized leash-holder bit and the
/// harness item tag are independent gates in vanilla; bbb projects the latter from the synchronized
/// `minecraft:item` registry tags rather than inferring it from the equipment asset.
pub(in crate::entity_models) fn render_happy_ghast_ropes_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !instance.render_state.kind.happy_ghast().is_leash_holder
        || !instance
            .render_state
            .kind
            .happy_ghast()
            .body_item_is_harness
    {
        return;
    }
    let EntityModelKind::HappyGhast { baby } = instance.kind else {
        return;
    };

    let model = happy_ghast_model(
        if baby {
            ModelLayer::HAPPY_GHAST_BABY_ROPES
        } else {
            ModelLayer::HAPPY_GHAST_ROPES
        },
        instance,
    );
    let pass = equipment_layer_pass(
        EntityModelLayerKind::InheritOverlay,
        EntityModelLayerRenderType::EntityCutout,
        HAPPY_GHAST_ROPES_TEXTURE_REF,
        [1.0; 4],
        0,
    );
    render_textured_no_overlay_layer_pass(
        meshes,
        &model,
        entity_model_root_transform(instance),
        pass,
        atlas,
    );
}

/// Vanilla `WolfArmorLayer`: adult wolves with a body armor item render the `WOLF_BODY` equipment
/// asset over `AdultWolfModel(ModelLayers.WOLF_ARMOR)`, baked with `CubeDeformation(0.2)`. The
/// armadillo-scute asset has a white base layer plus a dye-only overlay; damaged armor then adds an
/// `armorTranslucent` crack texture. `EquipmentLayerRenderer` inserts an optional runtime-atlas
/// trim after all base/foil layers and before those cracks.
pub(in crate::entity_models) fn render_wolf_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if !matches!(instance.kind, EntityModelKind::Wolf { baby: false, .. }) {
        return;
    }
    let transform = entity_model_root_transform(instance);
    let model = wolf_armor_model(wolf_is_angry(instance.kind), instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.wolf().body_equipment_asset,
        EntityEquipmentLayerType::WolfBody,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.wolf().body_armor_dye,
        instance.render_state.kind.wolf().body_armor_foil,
        instance.render_state.kind.wolf().body_armor_trim,
        true,
        1,
        atlas,
        owned_entries,
        dynamic_texture_atlas,
        None,
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    );
    emit_wolf_body_armor_crack(meshes, instance, &model, transform, atlas)
}

fn wolf_armor_crackiness_texture_ref(
    crackiness: WolfArmorCrackiness,
) -> Option<EntityModelTextureRef> {
    match crackiness {
        WolfArmorCrackiness::None => None,
        WolfArmorCrackiness::Low => Some(WOLF_ARMOR_CRACKINESS_LOW_TEXTURE_REF),
        WolfArmorCrackiness::Medium => Some(WOLF_ARMOR_CRACKINESS_MEDIUM_TEXTURE_REF),
        WolfArmorCrackiness::High => Some(WOLF_ARMOR_CRACKINESS_HIGH_TEXTURE_REF),
    }
}

fn emit_wolf_body_armor_crack(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    model: &ModelPart,
    transform: Mat4,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let Some(texture) =
        wolf_armor_crackiness_texture_ref(instance.render_state.kind.wolf().body_armor_crackiness)
    else {
        return;
    };
    let pass = equipment_layer_pass(
        EntityModelLayerKind::NoOverlay,
        EntityModelLayerRenderType::ArmorTranslucent,
        texture,
        [1.0, 1.0, 1.0, 1.0],
        0,
    );
    render_textured_no_overlay_layer_pass(meshes, model, transform, pass, atlas);
}

fn wolf_is_angry(kind: EntityModelKind) -> bool {
    matches!(kind, EntityModelKind::Wolf { angry: true, .. })
}

/// Vanilla `SimpleEquipmentLayer` over `EquineSaddleModel` for horse, donkey, mule, skeleton-horse,
/// and zombie-horse saddles. The layer has no baby model, so baby equines skip it. The two bridle line
/// parts are visible only while `EquineRenderState.isRidden` is true.
pub(in crate::entity_models) fn render_equine_saddle_layer(
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
        .equine()
        .saddle_equipment_asset
        .is_none()
    {
        return;
    }

    let (model_layer, layer_type, order) = match instance.kind {
        EntityModelKind::Horse { baby: false, .. } => (
            ModelLayer::HORSE_SADDLE,
            EntityEquipmentLayerType::HorseSaddle,
            2,
        ),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Donkey,
            baby: false,
            ..
        } => (
            ModelLayer::DONKEY_SADDLE,
            EntityEquipmentLayerType::DonkeySaddle,
            0,
        ),
        EntityModelKind::Donkey {
            family: DonkeyModelFamily::Mule,
            baby: false,
            ..
        } => (
            ModelLayer::MULE_SADDLE,
            EntityEquipmentLayerType::MuleSaddle,
            0,
        ),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Skeleton,
            baby: false,
        } => (
            ModelLayer::SKELETON_HORSE_SADDLE,
            EntityEquipmentLayerType::SkeletonHorseSaddle,
            0,
        ),
        EntityModelKind::UndeadHorse {
            family: UndeadHorseModelFamily::Zombie,
            baby: false,
        } => (
            ModelLayer::ZOMBIE_HORSE_SADDLE,
            EntityEquipmentLayerType::ZombieHorseSaddle,
            0,
        ),
        _ => return,
    };
    let transform = entity_model_root_transform(instance);

    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.equine().saddle_equipment_asset,
        layer_type,
    ) else {
        return;
    };
    let model = equine_model(model_layer, EquineModelClass::Saddle, instance);
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.equine().saddle_dye,
        instance.render_state.kind.equine().saddle_foil,
        instance.render_state.kind.equine().saddle_trim,
        true,
        order,
        atlas,
        owned_entries,
        dynamic_texture_atlas,
        None,
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    );
}

/// Vanilla `StriderRenderer` `SimpleEquipmentLayer(STRIDER_SADDLE)`: a non-empty saddle item renders
/// `AdultStriderModel(ModelLayers.STRIDER_SADDLE)` with `strider_saddle/saddle.png`. The layer has no
/// baby model, so baby striders skip it.
pub(in crate::entity_models) fn render_strider_saddle_layer(
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
        .strider()
        .saddle_equipment_asset
        .is_none()
    {
        return;
    }
    if !matches!(instance.kind, EntityModelKind::Strider { baby: false, .. }) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let model = strider_model(false, instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.strider().saddle_equipment_asset,
        EntityEquipmentLayerType::StriderSaddle,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.strider().saddle_dye,
        instance.render_state.kind.strider().saddle_foil,
        instance.render_state.kind.strider().saddle_trim,
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

/// Vanilla `CamelRenderer` / `CamelHuskRenderer` `SimpleEquipmentLayer`: a non-empty saddle item
/// renders `CamelSaddleModel(ModelLayers.CAMEL*_SADDLE)` with the family-specific equipment texture.
/// The layer has no baby model, so baby camels skip it; camel husks are adult-only and always use the
/// adult saddle model.
pub(in crate::entity_models) fn render_camel_saddle_layer(
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
        .camel()
        .saddle_equipment_asset
        .is_none()
    {
        return;
    }

    let (model_layer, layer_type) = match instance.kind {
        EntityModelKind::Camel {
            family: CamelModelFamily::Camel,
            baby: false,
        } => (
            ModelLayer::CAMEL_SADDLE,
            EntityEquipmentLayerType::CamelSaddle,
        ),
        EntityModelKind::Camel {
            family: CamelModelFamily::CamelHusk,
            ..
        } => (
            ModelLayer::CAMEL_HUSK_SADDLE,
            EntityEquipmentLayerType::CamelHuskSaddle,
        ),
        _ => return,
    };

    let transform = entity_model_root_transform(instance);
    let model = camel_saddle_model(model_layer, instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.camel().saddle_equipment_asset,
        layer_type,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.camel().saddle_dye,
        instance.render_state.kind.camel().saddle_foil,
        instance.render_state.kind.camel().saddle_trim,
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

/// Vanilla `NautilusRenderer` / `ZombieNautilusRenderer` `SimpleEquipmentLayer(NAUTILUS_SADDLE)`:
/// a non-empty saddle item renders `NautilusSaddleModel(ModelLayers.NAUTILUS_SADDLE)` over adult
/// living nautilus and zombie nautilus. The layer has no baby model, so baby living nautilus skip it.
pub(in crate::entity_models) fn render_nautilus_saddle_layer(
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
        .nautilus()
        .saddle_equipment_asset
        .is_none()
    {
        return;
    }
    if !matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    ) {
        return;
    }

    let transform = entity_model_root_transform(instance);
    let model = nautilus_model(ModelLayer::NAUTILUS_SADDLE, instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.nautilus().saddle_equipment_asset,
        EntityEquipmentLayerType::NautilusSaddle,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.nautilus().saddle_dye,
        instance.render_state.kind.nautilus().saddle_foil,
        instance.render_state.kind.nautilus().saddle_trim,
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

/// Vanilla `NautilusRenderer` / `ZombieNautilusRenderer` `SimpleEquipmentLayer(NAUTILUS_BODY)`:
/// a non-empty nautilus body armor item renders `NautilusArmorModel(ModelLayers.NAUTILUS_ARMOR)` over
/// adult living nautilus and zombie nautilus. The layer has no baby model, so baby living nautilus
/// skip it. The shared equipment renderer inserts foil after the base armor and a runtime-atlas
/// trim after the foil, before the later saddle equipment layer.
pub(in crate::entity_models) fn render_nautilus_body_armor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    if !matches!(
        instance.kind,
        EntityModelKind::Nautilus { baby: false } | EntityModelKind::ZombieNautilus { .. }
    ) {
        return;
    }
    let transform = entity_model_root_transform(instance);
    let model = nautilus_model(ModelLayer::NAUTILUS_ARMOR, instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.nautilus().body_equipment_asset,
        EntityEquipmentLayerType::NautilusBody,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.nautilus().body_armor_dye,
        instance.render_state.kind.nautilus().body_armor_foil,
        instance.render_state.kind.nautilus().body_armor_trim,
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

pub(in crate::entity_models) fn render_llama_decor_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    owned_entries: &[EntityModelOwnedTextureAtlasEntry],
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::Llama {
        baby, has_chest, ..
    } = instance.kind
    else {
        return;
    };
    let model = llama_model(
        if baby {
            ModelLayer::LLAMA_BABY_DECOR
        } else {
            ModelLayer::LLAMA_DECOR
        },
        has_chest,
        instance,
    );
    let transform = entity_model_root_transform(instance);
    let Some(layers) = resolved_equipment_asset_layers(
        equipment_assets,
        instance.render_state.kind.llama().body_equipment_asset,
        EntityEquipmentLayerType::LlamaBody,
    ) else {
        return;
    };
    emit_equipment_asset_layers(
        meshes,
        layers,
        EntityModelLayerKind::NoOverlay,
        transform,
        instance.render_state.kind.llama().body_decor_dye,
        instance.render_state.kind.llama().body_decor_foil,
        instance.render_state.kind.llama().body_decor_trim,
        true,
        1,
        atlas,
        owned_entries,
        dynamic_texture_atlas,
        None,
        |mesh, uv, tint| {
            model.render_textured(mesh, transform, uv, tint);
        },
    );
}

// Vanilla `VillagerModel.createNoHatModel` clears the whole head subtree (VillagerModel.java:72-75).
const VILLAGER_NO_HAT_EXCLUDED_PARTS: [&str; 1] = ["head"];

pub(in crate::entity_models) fn render_villager_profession_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    match instance.kind {
        EntityModelKind::Villager { baby } => {
            let transform = entity_model_root_transform(instance);
            let model = villager_model(baby, instance);
            emit_villager_data_layers(
                meshes,
                &model,
                transform,
                baby,
                false,
                instance.render_state.kind.villager().model_data,
                atlas,
            );
        }
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::ZombieVillager,
            baby,
        } => {
            let transform = entity_model_root_transform(instance);
            let model =
                zombie_variant_model(ZombieVariantModelFamily::ZombieVillager, baby, instance);
            emit_villager_data_layers(
                meshes,
                &model,
                transform,
                baby,
                true,
                instance.render_state.kind.villager().model_data,
                atlas,
            );
        }
        _ => {}
    }
}

fn emit_villager_data_layers(
    meshes: &mut EntityModelTexturedMeshes,
    model: &ModelPart,
    transform: Mat4,
    baby: bool,
    zombie: bool,
    data: VillagerModelData,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let passes = if zombie {
        zombie_villager_data_textured_layer_passes(baby, data)
    } else {
        villager_data_textured_layer_passes(baby, data)
    };
    let Some(type_pass) = passes.first().copied() else {
        return;
    };
    emit_villager_profession_layer(
        meshes,
        model,
        transform,
        type_pass,
        !villager_type_hat_visible(data, zombie),
        atlas,
    );

    for pass in passes.iter().copied().skip(1) {
        emit_villager_profession_layer(meshes, model, transform, pass, false, atlas);
    }
}

fn emit_villager_profession_layer(
    meshes: &mut EntityModelTexturedMeshes,
    root: &ModelPart,
    transform: Mat4,
    pass: EntityModelLayerPass,
    no_hat: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let submit = textured_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        if no_hat {
            root.render_textured_excluding(
                mesh,
                submit.transform,
                entry.uv,
                submit.tint,
                "",
                &VILLAGER_NO_HAT_EXCLUDED_PARTS,
            );
        } else {
            root.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StuckBodyPlacement {
    InCube,
    OnSurface,
}

pub(in crate::entity_models) fn render_player_stuck_arrow_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let model = ModelLayer::ARROW.bake();
    render_player_stuck_body_layer(
        meshes,
        instance,
        atlas,
        &model,
        ARROW_TEXTURE_REF,
        instance.render_state.kind.player().arrow_count,
        StuckBodyPlacement::InCube,
        0.9,
    );
}

pub(in crate::entity_models) fn render_player_stinger_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let model = ModelLayer::BEE_STINGER.bake();
    render_player_stuck_body_layer(
        meshes,
        instance,
        atlas,
        &model,
        BEE_STINGER_TEXTURE_REF,
        instance.render_state.kind.player().stinger_count,
        StuckBodyPlacement::OnSurface,
        1.0,
    );
}

#[allow(clippy::too_many_arguments)]
fn render_player_stuck_body_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    stuck_model: &ModelPart,
    texture: EntityModelTextureRef,
    synchronized_count: i32,
    placement: StuckBodyPlacement,
    model_scale: f32,
) {
    let EntityModelKind::Player { skin, .. } = instance.kind else {
        return;
    };
    let count = usize::try_from(synchronized_count)
        .unwrap_or(0)
        .min(MAX_PLAYER_STUCK_ITEMS_PER_KIND);
    if count == 0 {
        return;
    }

    // Use the base body's pose for attachment transforms.
    let host = player_model(skin.is_slim(), instance);
    let mut random = LegacyRandomSource::with_seed(i64::from(instance.entity_id));
    let root = player_model_root_transform(instance);
    for _ in 0..count {
        let part_index = random.next_int(PLAYER_STUCK_BODY_PARTS.len() as i32) as usize;
        let part = PLAYER_STUCK_BODY_PARTS[part_index];
        let Some(cube_count) = host.try_child_cube_count(part) else {
            continue;
        };
        let Ok(cube_bound) = i32::try_from(cube_count) else {
            continue;
        };
        if cube_bound == 0 {
            continue;
        }
        // Even every vanilla player base part's one-cube list consumes nextInt(1).
        let cube_index = random.next_int(cube_bound) as usize;
        let Some(attachment) = host.try_child_cube_attachment(part, cube_index) else {
            continue;
        };
        let mut mid = [
            random.next_float(),
            random.next_float(),
            random.next_float(),
        ];
        if placement == StuckBodyPlacement::OnSurface {
            let plane = random.next_int(3) as usize;
            mid[plane] = if mid[plane] > 0.5 { 1.0 } else { 0.5 };
        }
        let point = Vec3::from_array(std::array::from_fn(|axis| {
            (attachment.min[axis] + mid[axis] * (attachment.max[axis] - attachment.min[axis]))
                / 16.0
        }));
        let direction = Vec3::new(
            -(mid[0] * 2.0 - 1.0),
            -(mid[1] * 2.0 - 1.0),
            -(mid[2] * 2.0 - 1.0),
        );
        let direction_xz = (direction.x * direction.x + direction.z * direction.z).sqrt();
        let y_rot = direction.x.atan2(direction.z) - std::f32::consts::FRAC_PI_2;
        let z_rot = direction.y.atan2(direction_xz);
        let transform = root
            * attachment.transform
            * Mat4::from_translation(point)
            * Mat4::from_rotation_y(y_rot)
            * Mat4::from_rotation_z(z_rot)
            * Mat4::from_scale(Vec3::splat(model_scale));
        let submit = no_overlay_submission(
            meshes,
            EntityModelLayerRenderType::EntityCutoutCull,
            texture,
            [1.0; 4],
            transform,
            0,
        );
        render_textured_submission(meshes, submit, atlas, |mesh, entry| {
            stuck_model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
        });
    }
}

pub(in crate::entity_models) fn render_player_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    skin: EntityPlayerSkin,
    parts: PlayerModelPartVisibility,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    // The unified `PlayerModel` tree drives both render paths; `setup_anim` looks the head, runs the
    // inherited `HumanoidModel` walk swing + idle arm bob, and applies the crouch sneaking pose. The
    // six skin overlay parts (hat/jacket/sleeves/pants) are toggled by the player's part visibility
    // after `prepare` (the colored fallback shows every overlay). Held-item/attack/swim arm poses still
    // defer; the profile cape and WingsLayer emit as separate layers after the base body.
    let transform = player_model_root_transform(instance);
    let slim = skin.is_slim();
    let mut model = player_model(slim, instance);
    apply_player_part_visibility(&mut model, parts);
    let texture = default_player_skin_texture_ref(skin.fallback());
    if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = skin {
        let passes = player_textured_layer_passes_with_texture(parts, texture);
        render_textured_pass_with_dynamic_player_skin(
            meshes,
            &model,
            transform,
            passes[0],
            dynamic_player_skin,
            atlas,
            dynamic_player_skin_atlas,
        );
        return;
    }
    render_textured_layers(
        meshes,
        &model,
        transform,
        player_textured_layer_passes_with_texture(parts, texture),
        atlas,
    );
}

pub(in crate::entity_models) fn render_player_extra_ears_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    dynamic_player_skin_atlas: Option<&EntityDynamicPlayerSkinAtlasLayout>,
) {
    let EntityModelKind::Player { skin, .. } = instance.kind else {
        return;
    };
    if !instance.render_state.kind.player().show_extra_ears || instance.render_state.invisible {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }

    let transform = player_model_root_transform(instance);
    let model = player_ears_model(instance);
    let texture = default_player_skin_texture_ref(skin.fallback());
    let pass = player_extra_ears_layer_pass_with_texture(texture);
    let submit = textured_layer_submission(meshes, pass, transform);

    let submit = if let EntityPlayerSkin::Dynamic(dynamic_player_skin) = skin {
        if dynamic_player_skin.status == EntityDynamicPlayerSkinStatus::Ready {
            if let Some(entry) = dynamic_player_skin_atlas_entry(
                dynamic_player_skin_atlas,
                dynamic_player_skin.handle,
            ) {
                render_textured_dynamic_player_skin_submission(
                    meshes,
                    submit,
                    entry,
                    |mesh, entry| {
                        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
                    },
                );
                return;
            }
        }
        submit
    } else {
        submit
    };

    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(in crate::entity_models) fn render_player_cape_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
    dynamic_player_texture_atlas: Option<&EntityDynamicPlayerTextureAtlasLayout>,
) {
    let EntityModelKind::Player { skin, parts } = instance.kind else {
        return;
    };
    if !parts.cape {
        return;
    }
    if meshes.current_force_transparent || meshes.current_outline_only {
        return;
    }
    if equipment_assets.has_layer(
        instance.render_state.chest_wings_equipment_asset,
        EntityEquipmentLayerType::Wings,
    ) {
        return;
    }
    let Some(cape_texture) = instance.render_state.kind.player().cape_texture else {
        return;
    };
    let Some(entry) =
        dynamic_player_texture_atlas_entry(dynamic_player_texture_atlas, cape_texture)
    else {
        return;
    };

    let root = player_model_root_transform(instance);
    let layer_transform = root * player_cape_chest_equipment_transform(instance, equipment_assets);
    let model = player_model(skin.is_slim(), instance);
    let Some(body_transform) = model.try_descendant_attach_transform(&["body"]) else {
        return;
    };
    let mut cape_model = ModelLayer::PLAYER_CAPE.bake();
    let cape = cape_model.part_mut("cape").unwrap();
    // `PlayerCapeModel.setupAnim` composes its `rotateBy` after the cape bind pose.
    let cape_transform = layer_transform
        * body_transform
        * part_pose_transform(cape.pose)
        * player_cape_animation_transform(instance);
    cape.pose = PartPose {
        offset: [0.0; 3],
        rotation: [0.0; 3],
    };
    let pass = player_cape_layer_pass();
    let submit = textured_layer_submission(meshes, pass, layer_transform);
    render_textured_dynamic_player_texture_submission(meshes, submit, entry, |mesh, entry| {
        cape.render_textured(mesh, cape_transform, entry.uv, submit.tint);
    });
}

fn player_cape_chest_equipment_transform(
    instance: &EntityModelInstance,
    equipment_assets: &ResolvedEntityEquipmentAssetCatalog,
) -> Mat4 {
    if equipment_assets.has_layer(
        instance.render_state.chest_wings_equipment_asset,
        EntityEquipmentLayerType::Humanoid,
    ) {
        Mat4::from_translation(Vec3::new(0.0, -0.053125, 0.06875))
    } else {
        Mat4::IDENTITY
    }
}

fn player_cape_animation_transform(instance: &EntityModelInstance) -> Mat4 {
    let state = &instance.render_state;
    let rotation = Quat::from_rotation_y(-std::f32::consts::PI)
        * Quat::from_rotation_x(
            (6.0 + state.kind.player().cape_lean / 2.0 + state.kind.player().cape_flap)
                .to_radians(),
        )
        * Quat::from_rotation_z((state.kind.player().cape_lean2 / 2.0).to_radians())
        * Quat::from_rotation_y((180.0 - state.kind.player().cape_lean2 / 2.0).to_radians());
    Mat4::from_quat(rotation)
}

pub(in crate::entity_models) fn render_player_spin_attack_effect_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Player { .. })
        || instance.render_state.auto_spin_age_ticks.is_none()
    {
        return;
    }

    let model = spin_attack_effect_model(instance);
    let pass = player_spin_attack_effect_layer_pass();
    let submit = textured_layer_submission(meshes, pass, player_model_root_transform(instance));
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

pub(in crate::entity_models) fn render_player_parrot_on_shoulder_layer(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
) {
    if !matches!(instance.kind, EntityModelKind::Player { .. }) {
        return;
    }
    if let Some(variant) = instance.render_state.kind.player().left_shoulder_parrot {
        emit_player_parrot_on_shoulder_submission(meshes, instance, atlas, variant, true);
    }
    if let Some(variant) = instance.render_state.kind.player().right_shoulder_parrot {
        emit_player_parrot_on_shoulder_submission(meshes, instance, atlas, variant, false);
    }
}

fn emit_player_parrot_on_shoulder_submission(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    atlas: &EntityModelTextureAtlasLayout,
    variant: ParrotModelVariant,
    is_left: bool,
) {
    let pass = player_parrot_on_shoulder_layer_pass(variant);
    if layer_pass_hidden_by_invisible(meshes, pass) {
        return;
    }

    let parrot_instance =
        EntityModelInstance::parrot(instance.entity_id, instance.position, 0.0, variant)
            .with(|s| s.kind.parrot_mut().on_shoulder = true)
            .with(|s| {
                (s.head_yaw, s.head_pitch) = (
                    instance.render_state.head_yaw,
                    instance.render_state.head_pitch,
                )
            })
            .with(|s| {
                (s.walk_animation_pos, s.walk_animation_speed) = (
                    instance.render_state.walk_animation_pos,
                    instance.render_state.walk_animation_speed,
                )
            })
            .with(|s| s.age_in_ticks = instance.render_state.age_in_ticks);
    let model = parrot_model(&parrot_instance);
    let transform = player_shoulder_parrot_transform(instance, is_left);
    let submit = textured_layer_submission(meshes, pass, transform);
    render_textured_submission(meshes, submit, atlas, |mesh, entry| {
        model.render_textured(mesh, submit.transform, entry.uv, submit.tint);
    });
}

fn player_shoulder_parrot_transform(instance: &EntityModelInstance, is_left: bool) -> Mat4 {
    player_model_root_transform(instance)
        * Mat4::from_translation(Vec3::new(
            if is_left { 0.4 } else { -0.4 },
            if instance.render_state.is_crouching {
                -1.3
            } else {
                -1.5
            },
            0.0,
        ))
}

pub(super) fn wings_layer_transform_and_baby(
    instance: &EntityModelInstance,
) -> Option<(Mat4, bool)> {
    let (root, baby) = match instance.kind {
        EntityModelKind::Player { .. } => (player_model_root_transform(instance), false),
        EntityModelKind::Zombie { baby } => (entity_model_root_transform(instance), baby),
        EntityModelKind::ZombieVariant { family, baby } => {
            (zombie_variant_root_transform(instance, family), baby)
        }
        EntityModelKind::Skeleton | EntityModelKind::SkeletonVariant { .. } => {
            (entity_model_root_transform(instance), false)
        }
        EntityModelKind::Piglin { baby, .. } => (entity_model_root_transform(instance), baby),
        EntityModelKind::ArmorStand { small, .. } => (entity_model_root_transform(instance), small),
        _ => return None,
    };
    Some((root * Mat4::from_translation(Vec3::Z * 0.125), baby))
}

pub(super) fn player_wings_profile_texture_override(
    instance: &EntityModelInstance,
) -> Option<EntityDynamicPlayerTexture> {
    let player_state = *instance.render_state.kind.player();
    let EntityModelKind::Player { parts, .. } = instance.kind else {
        return None;
    };
    player_state
        .elytra_texture
        .or_else(|| parts.cape.then_some(player_state.cape_texture).flatten())
}

fn player_profile_wings_texture_ref(texture: EntityDynamicPlayerTexture) -> EntityModelTextureRef {
    match texture.kind {
        super::super::catalog::EntityDynamicPlayerTextureKind::Cape => {
            PLAYER_PROFILE_CAPE_TEXTURE_REF
        }
        super::super::catalog::EntityDynamicPlayerTextureKind::Elytra => {
            PLAYER_PROFILE_ELYTRA_TEXTURE_REF
        }
    }
}

/// Vanilla `DonkeyRenderer`: the base `DonkeyModel` / `BabyDonkeyModel` pass.
pub(in crate::entity_models) fn render_donkey_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    family: DonkeyModelFamily,
    baby: bool,
    has_chest: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    let class = if baby {
        EquineModelClass::BabyDonkey
    } else {
        EquineModelClass::Adult
    };
    render_equine_base_layers(
        meshes,
        instance,
        donkey_model_layer(family, baby),
        class,
        &donkey_textured_layer_passes(family, baby, has_chest)[..1],
        atlas,
    );
}

/// Vanilla `HorseRenderer`: the coat pass, then the `HorseMarkingLayer` overlay of the same pose.
pub(in crate::entity_models) fn render_horse_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    variant: HorseColorVariant,
    baby: bool,
    markings: HorseMarkings,
    atlas: &EntityModelTextureAtlasLayout,
) {
    render_equine_base_layers(
        meshes,
        instance,
        horse_model_layer(baby),
        horse_model_class(baby),
        &horse_textured_layer_passes(variant, baby, markings),
        atlas,
    );
}

/// Vanilla `UndeadHorseRenderer`: the skeleton / zombie horse `HorseModel` pass.
pub(in crate::entity_models) fn render_undead_horse_textured_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    family: UndeadHorseModelFamily,
    baby: bool,
    atlas: &EntityModelTextureAtlasLayout,
) {
    render_equine_base_layers(
        meshes,
        instance,
        undead_horse_model_layer(family, baby),
        horse_model_class(baby),
        &undead_horse_textured_layer_passes(family, baby),
        atlas,
    );
}

fn horse_model_class(baby: bool) -> EquineModelClass {
    if baby {
        EquineModelClass::BabyHorse
    } else {
        EquineModelClass::Adult
    }
}

fn render_equine_base_layers(
    meshes: &mut EntityModelTexturedMeshes,
    instance: &EntityModelInstance,
    layer: ModelLayer,
    class: EquineModelClass,
    passes: &[EntityModelLayerPass],
    atlas: &EntityModelTextureAtlasLayout,
) {
    let model = equine_model(layer, class, instance);
    render_textured_layers(
        meshes,
        &model,
        entity_model_root_transform(instance),
        passes.iter().copied(),
        atlas,
    );
}
