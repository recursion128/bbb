//! Held-item attachment: world-space transforms that place an item on a posed entity model. The renderer
//! owns the model pose (the posed arm bone or head bone); the native layer resolves the item to quads and
//! applies the per-item display transform before baking the held-item mesh into the item-model pass.

use std::f32::consts::{FRAC_PI_2, PI};

use glam::{Mat4, Vec3};

use super::colored::{
    entity_model_root_transform, fox_model_root_transform,
    mesh_transformer_scaled_model_root_transform, player_model_root_transform,
    villager_adult_model_root_transform, wither_skeleton_model_root_transform,
    zombie_variant_root_transform, GIANT_SCALE,
};
use super::model::EntityModel;
use super::model_layers::{
    allay_arm_holding_x_rot, AllayModel, ArmorStandModel, CopperGolemModel, FoxModel, IllagerModel,
    PiglinModel, PlayerModel, SkeletonModel, VillagerModel, WanderingTraderModel, WitchModel,
    ZombieModel, ZombieVariantModel,
};
use super::{EntityModelInstance, EntityModelKind, SkeletonModelFamily};

const CUSTOM_HEAD_ITEM_SCALE: f32 = 0.625;
const CUSTOM_HEAD_SKULL_SCALE: f32 = 1.1875;
const PIGLIN_CUSTOM_HEAD_HORIZONTAL_SCALE: f32 = 1.001_953_1;
const VILLAGER_CUSTOM_HEAD_Y_OFFSET: f32 = -0.117_187_5;
const VILLAGER_CUSTOM_HEAD_SKULL_Y_OFFSET: f32 = -0.074_218_75;

#[derive(Debug, Clone, Copy)]
struct CustomHeadTransforms {
    y_offset: f32,
    skull_y_offset: f32,
    horizontal_scale: f32,
}

impl CustomHeadTransforms {
    const DEFAULT: Self = Self {
        y_offset: 0.0,
        skull_y_offset: 0.0,
        horizontal_scale: 1.0,
    };

    const PIGLIN: Self = Self {
        y_offset: 0.0,
        skull_y_offset: 0.0,
        horizontal_scale: PIGLIN_CUSTOM_HEAD_HORIZONTAL_SCALE,
    };

    const VILLAGER: Self = Self {
        y_offset: VILLAGER_CUSTOM_HEAD_Y_OFFSET,
        skull_y_offset: VILLAGER_CUSTOM_HEAD_SKULL_Y_OFFSET,
        horizontal_scale: 1.0,
    };
}

/// The model→world transform of the hand attach point for a humanoid's main (`right`) or off (`left`)
/// hand, or `None` if the instance is not a humanoid that holds items the standard way. Composes the
/// posed arm bone (vanilla `translateToHand` = root + arm `translateAndRotate`) with the
/// `ItemInHandLayer` hand offset (`rotX(-90°)·rotY(180°)·translate(±offsetX, offsetY, offsetZ)/16`)
/// and the main-hand spear attack item transform (`SpearAnimations.thirdPersonAttackItem`) when the
/// rendered player is mid-STAB swing. Kinetic weapon hold sway is still separate P1 work.
/// Vanilla `useBabyOffset` selects the offsets: adult `(1, 2, -10)`, baby `(0, 1, -4.5)`. The baby
/// humanoid families (zombie, zombie variants, piglin) bake their reduced proportions straight into an
/// explicit baby mesh (no part scale), so the baby attach uses the same `root · arm` formula on the baby
/// model with only the offset swapped. The caller applies the item's third-person display transform and
/// the `0..=16`→unit model on top.
pub fn humanoid_hand_attach_transform(
    instance: &EntityModelInstance,
    left_hand: bool,
) -> Option<Mat4> {
    let arm_name = if left_hand { "left_arm" } else { "right_arm" };
    let (arm_world, baby) = humanoid_arm_world_transform(instance, arm_name)?;
    Some(
        item_in_hand_layer_base_transform(arm_world, left_hand, baby)
            * item_in_hand_layer_item_transform(instance, left_hand),
    )
}

fn item_in_hand_layer_base_transform(arm_world: Mat4, left_hand: bool, baby: bool) -> Mat4 {
    let sign = if left_hand { -1.0 } else { 1.0 };
    // Vanilla `ItemInHandLayer.submitArmWithItem`: `offsetX/Y/Z` are `(1, 2, -10)` adult, `(0, 1, -4.5)`
    // baby (so baby hands share the X=0 column — the left/right split comes only from the arm bone).
    let (offset_x, offset_y, offset_z) = if baby {
        (0.0, 1.0, -4.5)
    } else {
        (1.0, 2.0, -10.0)
    };
    arm_world
        * Mat4::from_rotation_x(-FRAC_PI_2)
        * Mat4::from_rotation_y(PI)
        * Mat4::from_translation(Vec3::new(
            sign * offset_x / 16.0,
            offset_y / 16.0,
            offset_z / 16.0,
        ))
}

fn item_in_hand_layer_item_transform(instance: &EntityModelInstance, left_hand: bool) -> Mat4 {
    if !left_hand
        && instance.render_state.main_hand_swing_is_stab
        && !instance.render_state.attack_arm_off_hand
    {
        spear_third_person_attack_item_transform(instance.render_state.attack_anim)
    } else {
        Mat4::IDENTITY
    }
}

/// Vanilla `SpearAnimations.thirdPersonAttackItem`: after the standard hand offset and before the item
/// stack submit, a STAB swing rotates the item around `(0, -0.125, 0.125)` by
/// `Axis.XN.rotationDegrees(70 * (attack - retract))`, then translates by the spear kinetic weapon's
/// fixed `forwardMovement = 0.38` along local Y.
fn spear_third_person_attack_item_transform(attack_anim: f32) -> Mat4 {
    if attack_anim <= 0.0 {
        return Mat4::IDENTITY;
    }
    let attack = progress(attack_anim, 0.05, 0.2).powi(2);
    let retract = ease_in_out_expo(progress(attack_anim, 0.4, 1.0));
    let amount = attack - retract;
    rotate_around(
        Vec3::new(0.0, -0.125, 0.125),
        Mat4::from_rotation_x(-(70.0 * amount).to_radians()),
    ) * Mat4::from_translation(Vec3::new(0.0, 0.38 * amount, 0.0))
}

fn rotate_around(pivot: Vec3, rotation: Mat4) -> Mat4 {
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
}

fn progress(value: f32, start: f32, end: f32) -> f32 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}

fn ease_in_out_expo(x: f32) -> f32 {
    if x < 0.5 {
        if x == 0.0 {
            0.0
        } else {
            2.0_f32.powf(20.0 * x - 10.0) / 2.0
        }
    } else if x == 1.0 {
        1.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * x + 10.0)) / 2.0
    }
}

/// The model→world transform used by vanilla `CopperGolemModel.translateToHand` plus
/// `ItemInHandLayer.submitArmWithItem`. bbb currently projects the renderer's steady `IDLE` hand branch:
/// `root -> body -> arm`, a ±90° Y rotation (right/left), a small forward offset, then the standard
/// third-person hand rotation and adult hand offset. Interaction keyframe hand placement stays deferred
/// with the copper golem interaction animations.
pub fn copper_golem_hand_attach_transform(
    instance: &EntityModelInstance,
    left_hand: bool,
) -> Option<Mat4> {
    let EntityModelKind::CopperGolem { .. } = instance.kind else {
        return None;
    };

    let arm_name = if left_hand { "left_arm" } else { "right_arm" };
    let mut model = CopperGolemModel::new();
    model.prepare(instance);
    let arm_world = entity_model_root_transform(*instance)
        * model
            .root()
            .try_descendant_attach_transform(&["body", arm_name])?;
    let sign = if left_hand { -1.0 } else { 1.0 };
    let hand_y_rot = if left_hand { FRAC_PI_2 } else { -FRAC_PI_2 };
    Some(
        arm_world
            * Mat4::from_rotation_y(hand_y_rot)
            * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.125))
            * Mat4::from_rotation_x(-FRAC_PI_2)
            * Mat4::from_rotation_y(PI)
            * Mat4::from_translation(Vec3::new(sign / 16.0, 2.0 / 16.0, -10.0 / 16.0)),
    )
}

/// The model->world transform used by vanilla `AllayModel.translateToHand` plus
/// `ItemInHandLayer.submitArmWithItem`. Unlike humanoids, the allay ignores the `arm` parameter in
/// `translateToHand`: both hand item states first walk `root -> body`, offset forward/up, rotate only
/// by `right_arm.xRot`, scale to `0.7`, and then the standard ItemInHandLayer left/right offset splits
/// the submitted item.
pub fn allay_hand_attach_transform(
    instance: &EntityModelInstance,
    left_hand: bool,
) -> Option<Mat4> {
    let EntityModelKind::Allay = instance.kind else {
        return None;
    };

    let mut model = AllayModel::new();
    model.prepare(instance);
    let body = entity_model_root_transform(*instance)
        * model
            .root()
            .try_descendant_attach_transform(&["root", "body"])?;
    let arm_x = allay_arm_holding_x_rot(
        instance.render_state.walk_animation_speed,
        instance.render_state.allay_holding_item_progress,
    );
    let sign = if left_hand { -1.0 } else { 1.0 };
    Some(
        body * Mat4::from_translation(Vec3::new(0.0, 1.0 / 16.0, 3.0 / 16.0))
            * Mat4::from_rotation_x(arm_x)
            * Mat4::from_scale(Vec3::splat(0.7))
            * Mat4::from_translation(Vec3::new(1.0 / 16.0, 0.0, 0.0))
            * Mat4::from_rotation_x(-FRAC_PI_2)
            * Mat4::from_rotation_y(PI)
            * Mat4::from_translation(Vec3::new(sign / 16.0, 2.0 / 16.0, -10.0 / 16.0)),
    )
}

/// The model→world transform used by vanilla `CustomHeadLayer` for non-skull, non-armor head-slot
/// items. The native item path applies the stack's retained `ItemDisplayContext.HEAD` transform after
/// this matrix.
pub fn custom_head_item_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    let (root, head, transforms) = custom_head_item_base_transform(instance)?;
    Some(custom_head_item_layer_transform(root, head, transforms))
}

/// The model->world transform used by vanilla `CustomHeadLayer` for skull block items in the HEAD
/// equipment slot. Unlike the generic item branch, the skull branch calls `SkullBlockRenderer`
/// directly, so it does not apply the item display rotation or the negative item-model scale.
pub(in crate::entity_models) fn custom_head_skull_transform(
    instance: &EntityModelInstance,
) -> Option<Mat4> {
    let (root, head, transforms) = custom_head_item_base_transform(instance)?;
    Some(custom_head_skull_layer_transform(root, head, transforms))
}

fn custom_head_item_layer_transform(
    root: Mat4,
    head: Mat4,
    transforms: CustomHeadTransforms,
) -> Mat4 {
    root * Mat4::from_scale(Vec3::new(
        transforms.horizontal_scale,
        1.0,
        transforms.horizontal_scale,
    )) * head
        * Mat4::from_translation(Vec3::new(0.0, -0.25 + transforms.y_offset, 0.0))
        * Mat4::from_rotation_y(PI)
        * Mat4::from_scale(Vec3::new(
            CUSTOM_HEAD_ITEM_SCALE,
            -CUSTOM_HEAD_ITEM_SCALE,
            -CUSTOM_HEAD_ITEM_SCALE,
        ))
}

fn custom_head_skull_layer_transform(
    root: Mat4,
    head: Mat4,
    transforms: CustomHeadTransforms,
) -> Mat4 {
    root * Mat4::from_scale(Vec3::new(
        transforms.horizontal_scale,
        1.0,
        transforms.horizontal_scale,
    )) * head
        * Mat4::from_translation(Vec3::new(0.0, transforms.skull_y_offset, 0.0))
        * Mat4::from_scale(Vec3::splat(CUSTOM_HEAD_SKULL_SCALE))
}

fn custom_head_item_base_transform(
    instance: &EntityModelInstance,
) -> Option<(Mat4, Mat4, CustomHeadTransforms)> {
    match instance.kind {
        EntityModelKind::Player { skin, .. } => {
            let slim = skin.is_slim();
            let mut model = PlayerModel::new(slim);
            model.prepare(instance);
            Some((
                player_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::Zombie { baby } => {
            let mut model = ZombieModel::new(baby);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::ZombieVariant { family, baby } => {
            let mut model = ZombieVariantModel::new(family, baby);
            model.prepare(instance);
            Some((
                zombie_variant_root_transform(*instance, family, baby),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::Piglin { family, baby } => {
            let mut model = PiglinModel::new(family, baby);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::PIGLIN,
            ))
        }
        EntityModelKind::Skeleton => {
            let mut model = SkeletonModel::new(None);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::SkeletonVariant { family } => {
            let mut model = SkeletonModel::new(Some(family));
            model.prepare(instance);
            let root = if family == SkeletonModelFamily::WitherSkeleton {
                wither_skeleton_model_root_transform(*instance)
            } else {
                entity_model_root_transform(*instance)
            };
            Some((
                root,
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::Illager { family } => {
            let mut model = IllagerModel::new(instance, family);
            model.prepare(instance);
            Some((
                villager_adult_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::Villager { baby } => {
            let mut model = VillagerModel::new(baby);
            model.prepare(instance);
            let root = if baby {
                entity_model_root_transform(*instance)
            } else {
                villager_adult_model_root_transform(*instance)
            };
            Some((
                root,
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::VILLAGER,
            ))
        }
        EntityModelKind::WanderingTrader => {
            let mut model = WanderingTraderModel::new();
            model.prepare(instance);
            Some((
                villager_adult_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::ArmorStand {
            small,
            show_arms,
            show_base_plate,
            pose,
            ..
        } => {
            let mut model = ArmorStandModel::new(small, show_arms, show_base_plate, pose);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance),
                model.root().try_child_attach_transform("head")?,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        EntityModelKind::CopperGolem { .. } => {
            let mut model = CopperGolemModel::new();
            model.prepare(instance);
            let head = model
                .root()
                .try_descendant_attach_transform(&["body", "head"])?
                * Mat4::from_translation(Vec3::new(0.0, 0.125, 0.0))
                * Mat4::from_scale(Vec3::splat(1.0625));
            Some((
                entity_model_root_transform(*instance),
                head,
                CustomHeadTransforms::DEFAULT,
            ))
        }
        _ => None,
    }
}

/// The model→world transform used by vanilla `CrossedArmsItemLayer` for villagers and wandering
/// traders before the held stack's `GROUND` display transform. The layer walks through the model's
/// combined `arms` part, then applies the shared crossed-arms rotation/scale/offset.
pub fn villager_crossed_arms_item_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    let (root, arms) = match instance.kind {
        EntityModelKind::Villager { baby } => {
            let mut model = VillagerModel::new(baby);
            model.prepare(instance);
            let root = if baby {
                entity_model_root_transform(*instance)
            } else {
                villager_adult_model_root_transform(*instance)
            };
            (root, model.root().try_child_attach_transform("arms")?)
        }
        EntityModelKind::WanderingTrader => {
            let mut model = WanderingTraderModel::new();
            model.prepare(instance);
            (
                villager_adult_model_root_transform(*instance),
                model.root().try_child_attach_transform("arms")?,
            )
        }
        _ => return None,
    };
    Some(
        root * arms
            * Mat4::from_rotation_x(0.75)
            * Mat4::from_scale(Vec3::splat(1.07))
            * Mat4::from_translation(Vec3::new(0.0, 0.13, -0.34))
            * Mat4::from_rotation_x(PI),
    )
}

/// The model→world transform used by vanilla `FoxHeldItemLayer` before the held stack's `GROUND`
/// display transform is applied. The layer manually translates to the already-posed fox head pivot, scales
/// baby held items by `0.75`, applies the head rotations, then offsets the item differently for
/// sleeping/non-sleeping adult and baby foxes before rotating the item upright.
pub fn fox_held_item_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    let EntityModelKind::Fox { baby, .. } = instance.kind else {
        return None;
    };

    let mut model = FoxModel::new(baby);
    model.prepare(instance);
    let head =
        fox_model_root_transform(*instance) * model.root().try_child_attach_transform("head")?;
    let sleeping = instance.render_state.fox_is_sleeping;
    let offset = match (baby, sleeping) {
        (true, true) => Vec3::new(0.4, 0.26, 0.15),
        (true, false) => Vec3::new(0.06, 0.26, -0.5),
        (false, true) => Vec3::new(0.46, 0.26, 0.22),
        (false, false) => Vec3::new(0.06, 0.27, -0.5),
    };
    let mut transform = head;
    if baby {
        transform *= Mat4::from_scale(Vec3::splat(0.75));
    }
    transform *= Mat4::from_translation(offset) * Mat4::from_rotation_x(FRAC_PI_2);
    if sleeping {
        transform *= Mat4::from_rotation_z(FRAC_PI_2);
    }
    Some(transform)
}

/// The model→world transform used by vanilla `DolphinCarryingItemLayer` before the carried stack's
/// `GROUND` display transform. The layer does not read `DolphinModel` parts; it stays in the entity root
/// frame and nudges the item from `(0, 1, -1)` based only on `state.xRot`. The baby dolphin's `0.5`
/// geometry scale is baked into `ModelLayers.DOLPHIN_BABY`, so this layer intentionally uses the unscaled
/// entity root transform.
pub fn dolphin_carried_item_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    let EntityModelKind::Dolphin { .. } = instance.kind else {
        return None;
    };
    let pitch = instance.render_state.head_pitch;
    let angle_x_percent = pitch.abs() / 60.0;
    let offset = if pitch < 0.0 {
        Vec3::new(
            0.0,
            1.0 - angle_x_percent * 0.5,
            -1.0 + angle_x_percent * 0.5,
        )
    } else {
        Vec3::new(
            0.0,
            1.0 + angle_x_percent * 0.8,
            -1.0 + angle_x_percent * 0.2,
        )
    };
    Some(entity_model_root_transform(*instance) * Mat4::from_translation(offset))
}

/// The model→world transform used by vanilla `PandaHoldsItemLayer` before the main-hand stack's `GROUND`
/// display transform. The layer stays in entity-root space: it renders only while the panda is sitting and
/// not scared, then offsets the stack to `(0.1, 1.4, -0.6)` with the eating bob applied directly to Y/Z.
pub fn panda_held_item_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    let EntityModelKind::Panda { .. } = instance.kind else {
        return None;
    };
    if !instance.render_state.panda_sitting || instance.render_state.panda_scared {
        return None;
    }

    let mut z = -0.6;
    let mut y = 1.4;
    if instance.render_state.panda_eating {
        let bob = (instance.render_state.age_in_ticks * 0.6).sin();
        z -= 0.2 * bob + 0.2;
        y -= 0.09 * bob;
    }
    Some(entity_model_root_transform(*instance) * Mat4::from_translation(Vec3::new(0.1, y, z)))
}

/// The model→world transform used by vanilla `WitchItemLayer` before the held stack's `GROUND` display
/// transform. Non-potion items use `CrossedArmsItemLayer` (`root -> arms`, then the crossed-arms
/// rotation/scale/offset). Potions use the special drinking branch: `root -> head -> nose`, then the
/// nose-local potion rotations. The model is prepared first so `isHoldingItem` pins the nose before the
/// potion branch reads it.
pub fn witch_held_item_transform(instance: &EntityModelInstance) -> Option<Mat4> {
    let EntityModelKind::Witch = instance.kind else {
        return None;
    };
    if !instance.render_state.witch_holding_item {
        return None;
    }

    let mut model = WitchModel::new();
    model.prepare(instance);
    let root = villager_adult_model_root_transform(*instance);
    let local = if instance.render_state.witch_holding_potion {
        model
            .root()
            .try_descendant_attach_transform(&["head", "nose"])?
            * Mat4::from_translation(Vec3::new(0.0625, 0.25, 0.0))
            * Mat4::from_rotation_z(PI)
            * Mat4::from_rotation_x(140.0_f32.to_radians())
            * Mat4::from_rotation_z(10.0_f32.to_radians())
            * Mat4::from_rotation_x(PI)
    } else {
        model.root().try_descendant_attach_transform(&["arms"])?
            * Mat4::from_rotation_x(0.75)
            * Mat4::from_scale(Vec3::splat(1.07))
            * Mat4::from_translation(Vec3::new(0.0, 0.13, -0.34))
            * Mat4::from_rotation_x(PI)
    };
    Some(root * local)
}

/// The world transform of a named arm bone plus whether the instance is a baby (so the caller picks the
/// baby hand offset), for the humanoid families that render held items: builds and poses the same model
/// + root transform the entity scene uses, then reads `root · arm` (vanilla `translateToHand`). Returns
/// `None` for non-humanoid kinds or any model that lacks the standard arm bone (so the held-item layer
/// degrades to rendering nothing rather than panicking).
fn humanoid_arm_world_transform(
    instance: &EntityModelInstance,
    arm_name: &str,
) -> Option<(Mat4, bool)> {
    match instance.kind {
        EntityModelKind::Player { skin, .. } => {
            let slim = skin.is_slim();
            let mut model = PlayerModel::new(slim);
            model.prepare(instance);
            Some((
                player_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::Zombie { baby } => {
            let mut model = ZombieModel::new(baby);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                baby,
            ))
        }
        EntityModelKind::Giant => {
            let mut model = ZombieModel::new(false);
            model.prepare(instance);
            Some((
                mesh_transformer_scaled_model_root_transform(*instance, GIANT_SCALE)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::ZombieVariant { family, baby } => {
            let mut model = ZombieVariantModel::new(family, baby);
            model.prepare(instance);
            Some((
                zombie_variant_root_transform(*instance, family, baby)
                    * model.root().try_child_attach_transform(arm_name)?,
                baby,
            ))
        }
        EntityModelKind::Piglin { family, baby } => {
            let mut model = PiglinModel::new(family, baby);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                baby,
            ))
        }
        EntityModelKind::Skeleton => {
            let mut model = SkeletonModel::new(None);
            model.prepare(instance);
            Some((
                entity_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::SkeletonVariant { family } => {
            let mut model = SkeletonModel::new(Some(family));
            model.prepare(instance);
            let root = if family == SkeletonModelFamily::WitherSkeleton {
                wither_skeleton_model_root_transform(*instance)
            } else {
                entity_model_root_transform(*instance)
            };
            Some((
                root * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        EntityModelKind::Illager { family } => {
            let mut model = IllagerModel::new(instance, family);
            model.prepare(instance);
            Some((
                villager_adult_model_root_transform(*instance)
                    * model.root().try_child_attach_transform(arm_name)?,
                false,
            ))
        }
        // Armor stands hold items on their posed arm bone (vanilla `ArmorStandRenderer`'s
        // `ItemInHandLayer`); `useBabyOffset` is false for ARMOR_STAND, so both full and small stands take
        // the adult offset. `ModelLayers.ARMOR_STAND_SMALL` applies `HumanoidModel.BABY_TRANSFORMER`, so
        // the arm part carries a 0.5 local scale that the held item must ride after the arm transform.
        EntityModelKind::ArmorStand {
            small,
            show_arms,
            show_base_plate,
            pose,
            ..
        } => {
            let mut model = ArmorStandModel::new(small, show_arms, show_base_plate, pose);
            model.prepare(instance);
            let mut arm_world = entity_model_root_transform(*instance)
                * model.root().try_child_attach_transform(arm_name)?;
            if small {
                arm_world *= Mat4::from_scale(Vec3::splat(0.5));
            }
            Some((arm_world, false))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_models::{
        CopperGolemWeathering, IllagerModelFamily, PiglinModelFamily, ZombieVariantModelFamily,
        DEFAULT_ARMOR_STAND_MODEL_POSE,
    };
    use crate::entity_models::{FoxModelVariant, PLAYER_MODEL_PARTS_ALL_VISIBLE};

    fn player_instance(y_rot: f32) -> EntityModelInstance {
        EntityModelInstance::player_with_parts(
            1,
            [0.0, 64.0, 0.0],
            y_rot,
            false,
            PLAYER_MODEL_PARTS_ALL_VISIBLE,
        )
    }

    #[test]
    fn non_humanoid_instances_have_no_hand_attach() {
        let creeper = EntityModelInstance::new(2, EntityModelKind::Creeper, [0.0, 0.0, 0.0], 0.0);
        assert!(humanoid_hand_attach_transform(&creeper, false).is_none());
    }

    #[test]
    fn custom_head_item_transform_covers_vanilla_custom_head_models() {
        let instances = [
            EntityModelInstance::player_with_parts(
                10,
                [0.0, 64.0, 0.0],
                0.0,
                false,
                PLAYER_MODEL_PARTS_ALL_VISIBLE,
            ),
            EntityModelInstance::new(
                11,
                EntityModelKind::Zombie { baby: false },
                [0.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::new(
                12,
                EntityModelKind::ZombieVariant {
                    family: ZombieVariantModelFamily::ZombieVillager,
                    baby: false,
                },
                [0.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::new(13, EntityModelKind::Skeleton, [0.0, 64.0, 0.0], 0.0),
            EntityModelInstance::new(
                14,
                EntityModelKind::SkeletonVariant {
                    family: SkeletonModelFamily::WitherSkeleton,
                },
                [0.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::new(
                15,
                EntityModelKind::Piglin {
                    family: PiglinModelFamily::Piglin,
                    baby: false,
                },
                [0.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::new(
                16,
                EntityModelKind::Illager {
                    family: IllagerModelFamily::Pillager,
                },
                [0.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::villager(17, [0.0, 64.0, 0.0], 0.0, false),
            EntityModelInstance::wandering_trader(18, [0.0, 64.0, 0.0], 0.0),
            EntityModelInstance::new(
                19,
                EntityModelKind::ArmorStand {
                    small: false,
                    marker: false,
                    show_arms: true,
                    show_base_plate: true,
                    pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
                },
                [0.0, 64.0, 0.0],
                0.0,
            ),
            EntityModelInstance::new(
                20,
                EntityModelKind::CopperGolem {
                    weathering: CopperGolemWeathering::Unaffected,
                },
                [0.0, 64.0, 0.0],
                0.0,
            ),
        ];

        for instance in instances {
            let origin = custom_head_item_transform(&instance)
                .unwrap()
                .transform_point3(Vec3::ZERO);
            assert!(
                origin.is_finite(),
                "{:?} should expose a finite custom-head item transform",
                instance.kind
            );
        }

        assert!(custom_head_item_transform(&EntityModelInstance::new(
            21,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        ))
        .is_none());
    }

    #[test]
    fn custom_head_item_transform_follows_the_posed_head() {
        let zombie = EntityModelInstance::new(
            22,
            EntityModelKind::Zombie { baby: false },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let resting = custom_head_item_transform(&zombie)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let looking = custom_head_item_transform(&zombie.with_head_look(45.0, -20.0))
            .unwrap()
            .transform_point3(Vec3::ZERO);

        assert!(looking.is_finite());
        assert_ne!(
            resting, looking,
            "CustomHeadLayer walks through the already-posed head bone"
        );
    }

    #[test]
    fn custom_head_layer_applies_villager_and_piglin_transforms() {
        let default_origin = custom_head_item_layer_transform(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            CustomHeadTransforms::DEFAULT,
        )
        .transform_point3(Vec3::ZERO);
        let villager_origin = custom_head_item_layer_transform(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            CustomHeadTransforms::VILLAGER,
        )
        .transform_point3(Vec3::ZERO);
        assert!(
            (villager_origin.y - default_origin.y - VILLAGER_CUSTOM_HEAD_Y_OFFSET).abs() < 1e-6
        );

        let piglin_x = custom_head_item_layer_transform(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            CustomHeadTransforms::PIGLIN,
        )
        .transform_vector3(Vec3::X)
        .length();
        assert!(
            (piglin_x - CUSTOM_HEAD_ITEM_SCALE * PIGLIN_CUSTOM_HEAD_HORIZONTAL_SCALE).abs() < 1e-6
        );
    }

    #[test]
    fn custom_head_skull_layer_uses_skull_offsets_and_scale() {
        let default_origin = custom_head_skull_layer_transform(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            CustomHeadTransforms::DEFAULT,
        )
        .transform_point3(Vec3::ZERO);
        let villager_origin = custom_head_skull_layer_transform(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            CustomHeadTransforms::VILLAGER,
        )
        .transform_point3(Vec3::ZERO);
        assert!(
            (villager_origin.y - default_origin.y - VILLAGER_CUSTOM_HEAD_SKULL_Y_OFFSET).abs()
                < 1e-6
        );

        let skull_x = custom_head_skull_layer_transform(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            CustomHeadTransforms::DEFAULT,
        )
        .transform_vector3(Vec3::X)
        .length();
        assert!((skull_x - CUSTOM_HEAD_SKULL_SCALE).abs() < 1e-6);
    }

    #[test]
    fn fox_held_item_attaches_to_the_posed_head() {
        // Vanilla `FoxHeldItemLayer` reads the already-posed parent model head, so the carried item moves
        // with the fox's interested head roll rather than remaining at the bind-pose snout.
        let fox = EntityModelInstance::fox(21, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red);
        let resting = fox_held_item_transform(&fox)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let tilted = fox_held_item_transform(&fox.with_fox_head_roll_angle(0.35))
            .unwrap()
            .transform_point3(Vec3::ZERO);
        assert!(resting.is_finite());
        assert!(tilted.is_finite());
        assert_ne!(resting, tilted);
        assert!(fox_held_item_transform(&EntityModelInstance::new(
            22,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0
        ))
        .is_none());
    }

    #[test]
    fn fox_held_item_follows_faceplant_renderer_pitch() {
        // `FoxHeldItemLayer` runs after `FoxRenderer.setupRotations` on the same pose stack. While
        // faceplanted, `FoxModel.setupAnim` suppresses the normal head look, so a pitch-only movement here
        // proves the layer picked up the renderer-level root `Rx(-state.xRot)`.
        let fox = EntityModelInstance::fox(25, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red)
            .with_fox_is_faceplanted(true)
            .with_age_in_ticks(0.0);
        let flat = fox_held_item_transform(&fox.with_head_look(0.0, 0.0))
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let pitched = fox_held_item_transform(&fox.with_head_look(0.0, 30.0))
            .unwrap()
            .transform_point3(Vec3::ZERO);
        assert_ne!(flat, pitched);
    }

    #[test]
    fn baby_fox_held_item_uses_the_layer_scale() {
        // `FoxHeldItemLayer` scales the baby-held stack by 0.75 after translating to the baby head pivot.
        // A unit vector therefore comes out shorter than the adult's even though both use the same item
        // display transform afterwards.
        let adult =
            EntityModelInstance::fox(23, [0.0, 64.0, 0.0], 0.0, false, FoxModelVariant::Red);
        let baby = EntityModelInstance::fox(24, [0.0, 64.0, 0.0], 0.0, true, FoxModelVariant::Red);
        let adult_x = fox_held_item_transform(&adult)
            .unwrap()
            .transform_vector3(Vec3::X)
            .length();
        let baby_x = fox_held_item_transform(&baby)
            .unwrap()
            .transform_vector3(Vec3::X)
            .length();
        assert!((adult_x - 1.0).abs() < 1e-6);
        assert!(
            (baby_x - 0.75).abs() < 1e-6,
            "baby item vector length {baby_x}"
        );
    }

    #[test]
    fn dolphin_carried_item_uses_x_rot_offset_without_baby_scale() {
        // Vanilla `DolphinCarryingItemLayer` leaves the item in entity-root space and only changes the
        // `(0, 1, -1)` offset by `abs(xRot) / 60`. The baby model's mesh transformer does not scale the
        // layer.
        let local_offset = |instance: EntityModelInstance| {
            let transform = dolphin_carried_item_transform(&instance).unwrap();
            let root = entity_model_root_transform(instance);
            (root.inverse() * transform).transform_point3(Vec3::ZERO)
        };
        let assert_close = |actual: Vec3, expected: Vec3| {
            assert!(
                (actual - expected).length() < 1e-5,
                "{actual:?} should be close to {expected:?}"
            );
        };
        let base = EntityModelInstance::dolphin(31, [0.0, 64.0, 0.0], 0.0, false);
        assert_close(local_offset(base), Vec3::new(0.0, 1.0, -1.0));
        assert_close(
            local_offset(base.with_head_look(0.0, -60.0)),
            Vec3::new(0.0, 0.5, -0.5),
        );
        assert_close(
            local_offset(base.with_head_look(0.0, 60.0)),
            Vec3::new(0.0, 1.8, -0.8),
        );
        let baby = EntityModelInstance::dolphin(32, [0.0, 64.0, 0.0], 0.0, true);
        assert_close(local_offset(baby), Vec3::new(0.0, 1.0, -1.0));
        assert!(dolphin_carried_item_transform(&EntityModelInstance::new(
            33,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0
        ))
        .is_none());
    }

    #[test]
    fn witch_held_item_uses_crossed_arms_or_potion_nose_branch() {
        // Vanilla `WitchItemLayer` uses the crossed-arms item layer for generic held items, but potions
        // attach to the already-posed nose after `WitchModel.setupAnim` pins it to the drinking pose.
        let base = EntityModelInstance::witch(41, [0.0, 64.0, 0.0], 0.0);
        assert!(witch_held_item_transform(&base).is_none());

        let generic = base.with_witch_holding_item(true);
        let generic_transform = witch_held_item_transform(&generic).unwrap();
        let potion_transform =
            witch_held_item_transform(&generic.with_witch_holding_potion(true)).unwrap();
        assert!(generic_transform.transform_point3(Vec3::ZERO).is_finite());
        assert!(potion_transform.transform_point3(Vec3::ZERO).is_finite());
        assert_ne!(
            generic_transform.transform_point3(Vec3::ZERO),
            potion_transform.transform_point3(Vec3::ZERO),
            "potion branch attaches to the nose, not the crossed arms"
        );
    }

    #[test]
    fn panda_held_item_uses_sitting_gate_and_eating_bob() {
        let local_offset = |instance: EntityModelInstance| {
            let transform = panda_held_item_transform(&instance).unwrap();
            let root = entity_model_root_transform(instance);
            (root.inverse() * transform).transform_point3(Vec3::ZERO)
        };
        let assert_close = |actual: Vec3, expected: Vec3| {
            assert!(
                (actual - expected).length() < 1e-5,
                "{actual:?} should be close to {expected:?}"
            );
        };

        let base = EntityModelInstance::panda(
            27,
            [0.0, 64.0, 0.0],
            0.0,
            false,
            crate::entity_models::PandaModelVariant::Normal,
        );
        assert!(panda_held_item_transform(&base).is_none());
        assert!(
            panda_held_item_transform(&base.with_panda_sitting(true).with_panda_scared(true))
                .is_none()
        );

        assert_close(
            local_offset(base.with_panda_sitting(true)),
            Vec3::new(0.1, 1.4, -0.6),
        );

        let eating = base
            .with_panda_sitting(true)
            .with_panda_eating(true)
            .with_age_in_ticks(5.0);
        let bob = (5.0_f32 * 0.6).sin();
        assert_close(
            local_offset(eating),
            Vec3::new(0.1, 1.4 - 0.09 * bob, -0.6 - 0.2 * bob - 0.2),
        );
        assert!(panda_held_item_transform(&EntityModelInstance::new(
            28,
            EntityModelKind::Creeper,
            [0.0, 64.0, 0.0],
            0.0,
        ))
        .is_none());
    }

    #[test]
    fn copper_golem_hand_attach_uses_held_item_arm_pose() {
        let base = EntityModelInstance::new(
            28,
            EntityModelKind::CopperGolem {
                weathering: CopperGolemWeathering::Unaffected,
            },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let resting = copper_golem_hand_attach_transform(&base, false).unwrap();
        let holding =
            copper_golem_hand_attach_transform(&base.with_copper_golem_holding_item(true), false)
                .unwrap();
        let left =
            copper_golem_hand_attach_transform(&base.with_copper_golem_holding_item(true), true)
                .unwrap();
        assert!(holding.transform_point3(Vec3::ZERO).is_finite());
        assert_ne!(
            resting.transform_point3(Vec3::ZERO),
            holding.transform_point3(Vec3::ZERO),
            "non-empty copper golem hands clamp the arms into the vanilla held-item pose"
        );
        assert_ne!(
            holding.transform_point3(Vec3::ZERO),
            left.transform_point3(Vec3::ZERO),
            "left and right hands use mirrored vanilla hand rotations"
        );
    }

    #[test]
    fn allay_hand_attach_matches_vanilla_translate_to_hand_shape() {
        // Vanilla `AllayModel.translateToHand` walks only `root -> body`, then applies a fixed
        // `(0, 1, 3)/16` offset, `right_arm.xRot`, `scale(0.7)`, and `(+1, 0, 0)/16` before the standard
        // `ItemInHandLayer` rotation and adult hand offset.
        let base = EntityModelInstance::allay(37, [0.0, 64.0, 0.0], 0.0)
            .with_age_in_ticks(9.0)
            .with_walk_animation(0.2, 0.3)
            .with_allay_holding_item_progress(1.0);
        let actual = allay_hand_attach_transform(&base, false).unwrap();

        let mut model = AllayModel::new();
        model.prepare(&base);
        let body = entity_model_root_transform(base)
            * model
                .root()
                .try_descendant_attach_transform(&["root", "body"])
                .unwrap();
        let expected = body
            * Mat4::from_translation(Vec3::new(0.0, 1.0 / 16.0, 3.0 / 16.0))
            * Mat4::from_rotation_x(allay_arm_holding_x_rot(
                base.render_state.walk_animation_speed,
                base.render_state.allay_holding_item_progress,
            ))
            * Mat4::from_scale(Vec3::splat(0.7))
            * Mat4::from_translation(Vec3::new(1.0 / 16.0, 0.0, 0.0))
            * Mat4::from_rotation_x(-FRAC_PI_2)
            * Mat4::from_rotation_y(PI)
            * Mat4::from_translation(Vec3::new(1.0 / 16.0, 2.0 / 16.0, -10.0 / 16.0));

        assert!(
            (actual.transform_point3(Vec3::ZERO) - expected.transform_point3(Vec3::ZERO)).length()
                < 1.0e-6
        );
        assert!(
            (actual.transform_vector3(Vec3::X) - expected.transform_vector3(Vec3::X)).length()
                < 1.0e-6
        );
    }

    #[test]
    fn allay_hand_attach_uses_right_arm_xrot_and_layer_side_offset() {
        let base = EntityModelInstance::allay(38, [0.0, 64.0, 0.0], 0.0)
            .with_age_in_ticks(4.0)
            .with_walk_animation(0.0, 0.0);
        let empty_right = allay_hand_attach_transform(&base, false).unwrap();
        let holding_right =
            allay_hand_attach_transform(&base.with_allay_holding_item_progress(1.0), false)
                .unwrap();
        let holding_left =
            allay_hand_attach_transform(&base.with_allay_holding_item_progress(1.0), true).unwrap();

        assert_ne!(
            empty_right.transform_point3(Vec3::ZERO),
            holding_right.transform_point3(Vec3::ZERO),
            "holdingAnimationProgress changes right_arm.xRot and moves the submitted item"
        );
        assert_ne!(
            holding_right.transform_point3(Vec3::ZERO),
            holding_left.transform_point3(Vec3::ZERO),
            "ItemInHandLayer's final left/right offset splits the allay hand item"
        );
        assert!(allay_hand_attach_transform(
            &EntityModelInstance::new(39, EntityModelKind::Creeper, [0.0, 64.0, 0.0], 0.0),
            false
        )
        .is_none());
    }

    #[test]
    fn villager_crossed_arms_item_transform_covers_baby_and_trader() {
        let adult = EntityModelInstance::villager(31, [0.0, 64.0, 0.0], 0.0, false);
        let baby = EntityModelInstance::villager(32, [0.0, 64.0, 0.0], 0.0, true);
        let trader = EntityModelInstance::wandering_trader(33, [0.0, 64.0, 0.0], 0.0);
        let adult_origin = villager_crossed_arms_item_transform(&adult)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let baby_origin = villager_crossed_arms_item_transform(&baby)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let trader_origin = villager_crossed_arms_item_transform(&trader)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        assert!(adult_origin.is_finite());
        assert!(baby_origin.is_finite());
        assert!(trader_origin.is_finite());
        assert_ne!(
            adult_origin, baby_origin,
            "baby villager uses BabyVillagerModel's own crossed-arms part"
        );
        assert!(
            villager_crossed_arms_item_transform(&EntityModelInstance::new(
                34,
                EntityModelKind::Skeleton,
                [0.0, 64.0, 0.0],
                0.0,
            ))
            .is_none()
        );
    }

    #[test]
    fn adult_humanoid_mobs_have_a_hand_attach() {
        // A zombie (and the other weapon-holding humanoid families) attaches a held item the same way a
        // player does — the generic dispatch poses the family model and reads its right-arm bone.
        let zombie = EntityModelInstance::new(
            3,
            EntityModelKind::Zombie { baby: false },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let attach = humanoid_hand_attach_transform(&zombie, false).unwrap();
        assert!(attach.transform_point3(Vec3::ZERO).is_finite());
    }

    #[test]
    fn giant_hand_attach_uses_the_scaled_zombie_model() {
        // Vanilla `GiantMobRenderer` adds the ordinary `ItemInHandLayer` over `GiantZombieModel`,
        // whose layer is `MeshTransformer.scaling(6.0)`. The hand basis therefore scales with the
        // body instead of using the adult zombie's unscaled arm space.
        let zombie = EntityModelInstance::new(
            35,
            EntityModelKind::Zombie { baby: false },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let giant = EntityModelInstance::giant(36, [0.0, 64.0, 0.0], 0.0);
        let zombie_attach = humanoid_hand_attach_transform(&zombie, false).unwrap();
        let giant_attach = humanoid_hand_attach_transform(&giant, false).unwrap();

        assert!(giant_attach.transform_point3(Vec3::ZERO).is_finite());
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let zombie_len = zombie_attach.transform_vector3(axis).length();
            let giant_len = giant_attach.transform_vector3(axis).length();
            assert!(
                (giant_len - zombie_len * GIANT_SCALE).abs() < 1.0e-5,
                "giant hand basis {giant_len} should be {GIANT_SCALE}x zombie basis {zombie_len}"
            );
        }
    }

    #[test]
    fn skeleton_held_item_follows_the_drawn_bow_aim() {
        // The held-item attach reads the SAME posed model as the body, so a skeleton drawing its bow
        // (`is_aggressive && main_hand_holds_bow`) raises the right hand from hanging at rest to the
        // horizontal `BOW_AND_ARROW` aim — the bow mesh tracks the aimed arm, no extra wiring.
        let skeleton =
            EntityModelInstance::new(8, EntityModelKind::Skeleton, [0.0, 64.0, 0.0], 0.0);
        let resting = humanoid_hand_attach_transform(&skeleton, false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let aiming = humanoid_hand_attach_transform(
            &skeleton
                .with_is_aggressive(true)
                .with_main_hand_holds_bow(true),
            false,
        )
        .unwrap()
        .transform_point3(Vec3::ZERO);
        assert!(aiming.is_finite());
        // The resting arm hangs down; the aimed arm swings up to horizontal, so the hand rises.
        assert!(
            aiming.y > resting.y + 0.2,
            "aimed hand {aiming:?} rises above the resting hand {resting:?}"
        );
        // The bow aim is gated on both flags: a holstered bow with no aggression keeps the resting hand.
        let holstered =
            humanoid_hand_attach_transform(&skeleton.with_main_hand_holds_bow(true), false)
                .unwrap()
                .transform_point3(Vec3::ZERO);
        assert_eq!(holstered, resting);
    }

    #[test]
    fn baby_humanoid_mobs_attach_lower_and_more_inward_than_adults() {
        // Baby zombies hold items too (vanilla `BabyZombieModel` is an explicit smaller mesh with the
        // baby `ItemInHandLayer` offset). The baby hand sits below and closer to the body center than the
        // adult's, since the baby arm bone is smaller and the baby offset drops X to 0.
        let adult = EntityModelInstance::new(
            5,
            EntityModelKind::Zombie { baby: false },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let baby = EntityModelInstance::new(
            6,
            EntityModelKind::Zombie { baby: true },
            [0.0, 64.0, 0.0],
            0.0,
        );
        let adult_hand = humanoid_hand_attach_transform(&adult, false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let baby_hand = humanoid_hand_attach_transform(&baby, false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        assert!(baby_hand.is_finite());
        // Baby is shorter, so its hand is lower than the adult's.
        assert!(
            baby_hand.y < adult_hand.y,
            "baby hand {baby_hand:?} below adult {adult_hand:?}"
        );
        // Baby right hand is closer to the X center than the adult's (smaller arm + X=0 offset).
        assert!(
            baby_hand.x.abs() < adult_hand.x.abs(),
            "baby hand {baby_hand:?} more inward than adult {adult_hand:?}"
        );
    }

    #[test]
    fn armor_stand_held_items_include_the_small_model_part_scale() {
        let stand = |small| {
            EntityModelInstance::new(
                7,
                EntityModelKind::ArmorStand {
                    small,
                    marker: false,
                    show_arms: true,
                    show_base_plate: true,
                    pose: DEFAULT_ARMOR_STAND_MODEL_POSE,
                },
                [0.0, 64.0, 0.0],
                0.0,
            )
        };
        // Armor stands always use the adult `ItemInHandLayer` offset, but the small model's arm part
        // carries `HumanoidModel.BABY_TRANSFORMER`'s 0.5 body scale and the held item must ride it.
        let adult = humanoid_hand_attach_transform(&stand(false), false).unwrap();
        let small = humanoid_hand_attach_transform(&stand(true), false).unwrap();
        assert!(adult.transform_point3(Vec3::ZERO).is_finite());
        assert!(small.transform_point3(Vec3::ZERO).is_finite());
        let adult_basis =
            (adult.transform_point3(Vec3::X) - adult.transform_point3(Vec3::ZERO)).length();
        let small_basis =
            (small.transform_point3(Vec3::X) - small.transform_point3(Vec3::ZERO)).length();
        assert!(
            (small_basis / adult_basis - 0.5).abs() < 1e-5,
            "small armor stand item scale {small_basis} should be half adult {adult_basis}"
        );
    }

    #[test]
    fn right_hand_attach_sits_in_front_of_and_below_the_shoulder() {
        // The attach point is to the right of the entity origin, below head height, and pushed forward
        // out of the body (the -10/16 hand offset along the arm).
        let attach = humanoid_hand_attach_transform(&player_instance(0.0), false).unwrap();
        let origin = attach.transform_point3(Vec3::ZERO);
        // Right hand → negative model-X side; the AvatarRenderer scale keeps it within ~1 block.
        assert!(origin.x < 0.0, "right hand on the -x side, got {origin:?}");
        assert!(
            origin.y < 64.0 + 1.6,
            "hand below head height, got {origin:?}"
        );
        assert!(origin.is_finite());
    }

    #[test]
    fn main_hand_spear_attack_applies_vanilla_item_layer_stab_transform() {
        let player = player_instance(0.0)
            .with_attack_anim(0.1)
            .with_main_hand_swing_is_stab(true);
        let (arm_world, baby) = humanoid_arm_world_transform(&player, "right_arm").unwrap();
        let expected = item_in_hand_layer_base_transform(arm_world, false, baby)
            * spear_third_person_attack_item_transform(0.1);
        let actual = humanoid_hand_attach_transform(&player, false).unwrap();
        assert_close_transform(actual, expected);

        let base_only = item_in_hand_layer_base_transform(arm_world, false, baby);
        let base_origin = base_only.transform_point3(Vec3::ZERO);
        let stabbed_origin = actual.transform_point3(Vec3::ZERO);
        assert!(
            (stabbed_origin - base_origin).length() > 0.01,
            "thirdPersonAttackItem should visibly move the submitted spear"
        );

        let (left_world, left_baby) = humanoid_arm_world_transform(&player, "left_arm").unwrap();
        let expected_left = item_in_hand_layer_base_transform(left_world, true, left_baby);
        assert_close_transform(
            humanoid_hand_attach_transform(&player, true).unwrap(),
            expected_left,
        );

        let whack = player.with_main_hand_swing_is_stab(false);
        let (whack_world, whack_baby) = humanoid_arm_world_transform(&whack, "right_arm").unwrap();
        assert_close_transform(
            humanoid_hand_attach_transform(&whack, false).unwrap(),
            item_in_hand_layer_base_transform(whack_world, false, whack_baby),
        );
    }

    #[test]
    fn left_and_right_hands_mirror_across_x() {
        let right = humanoid_hand_attach_transform(&player_instance(0.0), false)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        let left = humanoid_hand_attach_transform(&player_instance(0.0), true)
            .unwrap()
            .transform_point3(Vec3::ZERO);
        // Mirror entities face -Z; the two hands straddle the body center on X.
        assert!(
            right.x < left.x,
            "right {right:?} should be left of left {left:?}"
        );
    }

    fn assert_close_transform(actual: Mat4, expected: Mat4) {
        for (actual, expected) in actual
            .to_cols_array()
            .into_iter()
            .zip(expected.to_cols_array())
        {
            assert!(
                (actual - expected).abs() < 1.0e-5,
                "transform mismatch: {actual} != {expected}"
            );
        }
    }
}
