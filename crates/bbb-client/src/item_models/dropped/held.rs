use super::*;

/// The held-item residency submissions for this frame.
#[derive(Default)]
pub(crate) struct HeldItemModels {
    pub residency: ItemModelResidencyFrame,
}

/// Bakes the third-person main- and off-hand held items for every humanoid entity that holds one
/// (players and the weapon-holding mobs — zombies, giants, skeletons, piglins, illagers; vanilla
/// `ItemInHandLayer`) plus non-humanoid `ArmedModel` users such as allays and copper golems. The hand
/// attach transform comes from the renderer's posed model; native resolves the item to quads (block or
/// flat) and applies the item's third-person display transform.
#[cfg(test)]
pub(crate) fn held_item_models(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
) -> crate::item_models::test_meshes::ResidentMeshView<HeldItemModels> {
    let registry_keys = ItemRegistryKeySet::from_world(world);
    let models = held_item_models_with_variant_textures(
        instances,
        world,
        item_runtime,
        terrain_textures,
        None,
        &registry_keys,
    );
    crate::item_models::test_meshes::ResidentMeshView::new(models, |models| &models.residency)
}

pub(crate) fn held_item_models_with_variant_textures(
    instances: &[EntityModelInstance],
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    _terrain_textures: &TerrainTextureState,
    variant_textures: Option<&NativeEntityVariantTextureRuntime>,
    registry_keys: &ItemRegistryKeySet,
) -> HeldItemModels {
    let mut residency = ItemModelResidencyFrame::default();
    let Some(item_runtime) = item_runtime else {
        return HeldItemModels { residency };
    };
    let env = ItemBakeEnv {
        world,
        item_runtime,
        variant_textures,
        registry_keys: registry_keys.model_keys(),
    };

    for instance in instances {
        // Vanilla `ItemInHandLayer.submit` submits the right arm first and the left arm second. Each arm's
        // stack comes from `LivingEntity.getItemHeldByArm(arm)`, so a left-main-arm entity swaps which
        // logical hand slot is attached to the right/left arm.
        match instance.kind {
            EntityModelKind::Player { .. }
            | EntityModelKind::Zombie { .. }
            | EntityModelKind::ZombieVariant { .. }
            | EntityModelKind::Piglin { .. }
            | EntityModelKind::Skeleton
            | EntityModelKind::SkeletonVariant { .. }
            | EntityModelKind::Illager { .. }
            | EntityModelKind::ArmorStand { .. } => {
                bake_held_hand(instance, false, &env, &mut residency);
                bake_held_hand(instance, true, &env, &mut residency);
                bake_custom_head_item(instance, &env, &mut residency);
            }
            EntityModelKind::Giant => {
                bake_held_hand(instance, false, &env, &mut residency);
                bake_held_hand(instance, true, &env, &mut residency);
            }
            EntityModelKind::Villager { .. } | EntityModelKind::WanderingTrader => {
                bake_villager_crossed_arms_item(instance, &env, &mut residency);
                bake_custom_head_item(instance, &env, &mut residency);
            }
            EntityModelKind::CopperGolem { .. } => {
                bake_copper_golem_held_items(instance, &env, &mut residency);
                bake_custom_head_item(instance, &env, &mut residency);
            }
            EntityModelKind::Fox { .. } => bake_fox_held_item(instance, &env, &mut residency),
            EntityModelKind::Dolphin { .. } => {
                bake_dolphin_carried_item(instance, &env, &mut residency)
            }
            EntityModelKind::Witch => bake_witch_held_item(instance, &env, &mut residency),
            EntityModelKind::Allay => bake_allay_held_items(instance, &env, &mut residency),
            EntityModelKind::Panda { .. } => bake_panda_held_item(instance, &env, &mut residency),
            _ => {}
        }
    }

    HeldItemModels { residency }
}

/// Bakes one arm's held item onto its arm bone with that arm's own
/// `thirdperson_{left,right}hand` display transform. The logical hand slot is
/// resolved from the entity's main arm, mirroring vanilla `getItemHeldByArm`.
pub(super) fn bake_held_hand(
    instance: &EntityModelInstance,
    left_arm: bool,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let off_hand = left_arm != instance.render_state.main_arm_left;
    let Some(stack) = env.world.held_item(instance.entity_id, off_hand) else {
        return;
    };
    if !humanoid_item_in_hand_layer_visible(instance) {
        return;
    }

    // The item's own retained third-person transform for this arm (handheld tools angle the model,
    // blocks tilt it, generated items lay it flat); falls back to the parent-model default per path.
    let context = if left_arm {
        BlockModelDisplayContext::ThirdPersonLeftHand
    } else {
        BlockModelDisplayContext::ThirdPersonRightHand
    };
    let using_item = entity_hand_using_item(instance, off_hand);
    // Vanilla `PlayerItemInHandLayer.submitArmWithItem`: an avatar using a spyglass in this hand
    // outside a melee swing (`attackTime < 1e-5`) replaces the in-hand render with
    // `renderItemHeldToEye` — the stack resolved at `ItemDisplayContext.HEAD` with the same living
    // owner (`AvatarRenderer` fills `heldOnHead` only for `Items.SPYGLASS`) and attached to the
    // clamped head bone. Non-avatar kinds return no eye transform and keep the in-hand render,
    // matching the generic `ItemInHandLayer`.
    if using_item
        && instance.render_state.attack_anim < 1.0e-5
        && held_stack_is_spyglass(env.item_runtime, stack)
    {
        if let Some(eye) = player_held_to_eye_transform(instance, left_arm) {
            bake_item_stack(
                env,
                stack,
                BlockModelDisplayContext::Head,
                &entity_item_owner(env.world, instance, using_item),
                &ItemPlacement::attached_to_entity(
                    instance,
                    eye,
                    left_arm,
                    BLOCK_HEAD_FALLBACK,
                    GENERATED_HEAD_FALLBACK,
                ),
                residency,
            );
            return;
        }
    }
    for hand in humanoid_hand_attach_transforms(instance, left_arm) {
        bake_item_stack(
            env,
            stack,
            context,
            &entity_item_owner(env.world, instance, using_item),
            &ItemPlacement::attached_to_entity(
                instance,
                hand,
                left_arm,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
            ),
            residency,
        );
    }
}

pub(super) fn humanoid_item_in_hand_layer_visible(instance: &EntityModelInstance) -> bool {
    !matches!(
        instance.kind,
        EntityModelKind::Illager {
            family: IllagerModelFamily::Illusioner
        }
    ) || instance.render_state.kind.illager().spellcasting
        || instance.render_state.is_aggressive
}

/// Bakes a fox's main-hand stack in its mouth. Vanilla `FoxHeldItemLayer` is backed by
/// `HoldingEntityRenderState.extractHoldingEntityRenderState`, which resolves `entity.getMainHandItem()`
/// with `ItemDisplayContext.GROUND` before applying the fox-head layer transform.
pub(super) fn bake_fox_held_item(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(stack) = env.world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = fox_held_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack(
        env,
        stack,
        BlockModelDisplayContext::Ground,
        &entity_item_owner(env.world, instance, using_item),
        &ItemPlacement::attached_to_entity(
            instance,
            transform,
            false,
            BLOCK_GROUND_FALLBACK,
            GENERATED_GROUND_FALLBACK,
        ),
        residency,
    );
}

/// Bakes a dolphin's carried main-hand stack. Vanilla `DolphinCarryingItemLayer` shares
/// `HoldingEntityRenderState` with foxes, so it reads `entity.getMainHandItem()` resolved in the
/// `GROUND` display context and places it at a pitch-adjusted entity-root offset.
pub(super) fn bake_dolphin_carried_item(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(stack) = env.world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = dolphin_carried_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack(
        env,
        stack,
        BlockModelDisplayContext::Ground,
        &entity_item_owner(env.world, instance, using_item),
        &ItemPlacement::attached_to_entity(
            instance,
            transform,
            false,
            BLOCK_GROUND_FALLBACK,
            GENERATED_GROUND_FALLBACK,
        ),
        residency,
    );
}

/// Bakes a witch's main-hand stack through `WitchItemLayer`: potions attach to the posed nose, while all
/// other held items sit in the crossed arms. Vanilla resolves the stack through `HoldingEntityRenderState`
/// with `ItemDisplayContext.GROUND`.
pub(super) fn bake_witch_held_item(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(stack) = env.world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = witch_held_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack(
        env,
        stack,
        BlockModelDisplayContext::Ground,
        &entity_item_owner(env.world, instance, using_item),
        &ItemPlacement::attached_to_entity(
            instance,
            transform,
            false,
            BLOCK_GROUND_FALLBACK,
            GENERATED_GROUND_FALLBACK,
        ),
        residency,
    );
}

/// Bakes a copper golem's standard `ItemInHandLayer` hand items. Vanilla uses the normal third-person
/// left/right contexts, with `CopperGolemModel.translateToHand` supplying the posed arm transform.
pub(super) fn bake_copper_golem_held_items(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    for left_hand in [false, true] {
        let Some(stack) = env.world.held_item(instance.entity_id, left_hand) else {
            continue;
        };
        let Some(hand) = copper_golem_hand_attach_transform(instance, left_hand) else {
            continue;
        };
        let context = if left_hand {
            BlockModelDisplayContext::ThirdPersonLeftHand
        } else {
            BlockModelDisplayContext::ThirdPersonRightHand
        };
        let using_item = entity_hand_using_item(instance, left_hand);
        bake_item_stack(
            env,
            stack,
            context,
            &entity_item_owner(env.world, instance, using_item),
            &ItemPlacement::attached_to_entity(
                instance,
                hand,
                left_hand,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
            ),
            residency,
        );
    }
}

/// Bakes an allay's standard `ItemInHandLayer` hand items. Vanilla uses
/// `ArmedEntityRenderState.extractArmedEntityRenderState`, so the right/left item states use
/// third-person right/left hand contexts even though `AllayModel.translateToHand` itself ignores the
/// arm parameter before the shared layer offset.
pub(super) fn bake_allay_held_items(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    for left_hand in [false, true] {
        let Some(stack) = env.world.held_item(instance.entity_id, left_hand) else {
            continue;
        };
        let Some(hand) = allay_hand_attach_transform(instance, left_hand) else {
            continue;
        };
        let context = if left_hand {
            BlockModelDisplayContext::ThirdPersonLeftHand
        } else {
            BlockModelDisplayContext::ThirdPersonRightHand
        };
        let using_item = entity_hand_using_item(instance, left_hand);
        bake_item_stack(
            env,
            stack,
            context,
            &entity_item_owner(env.world, instance, using_item),
            &ItemPlacement::attached_to_entity(
                instance,
                hand,
                left_hand,
                BLOCK_THIRD_PERSON_FALLBACK,
                GENERATED_THIRD_PERSON_FALLBACK,
            ),
            residency,
        );
    }
}

/// Bakes the main-hand stack rendered by vanilla `CrossedArmsItemLayer` for villagers and wandering
/// traders. `HoldingEntityRenderState` resolves this layer with `ItemDisplayContext.GROUND`.
pub(super) fn bake_villager_crossed_arms_item(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(stack) = env.world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = villager_crossed_arms_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack(
        env,
        stack,
        BlockModelDisplayContext::Ground,
        &entity_item_owner(env.world, instance, using_item),
        &ItemPlacement::attached_to_entity(
            instance,
            transform,
            false,
            BLOCK_GROUND_FALLBACK,
            GENERATED_GROUND_FALLBACK,
        ),
        residency,
    );
}

/// Bakes the main-hand stack rendered by vanilla `PandaHoldsItemLayer`. `HoldingEntityRenderState`
/// resolves the stack in `ItemDisplayContext.GROUND`; the renderer transform already gates sitting/scared
/// and applies the eating bob.
pub(super) fn bake_panda_held_item(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(stack) = env.world.held_item(instance.entity_id, false) else {
        return;
    };
    let Some(transform) = panda_held_item_transform(instance) else {
        return;
    };
    let using_item = entity_hand_using_item(instance, false);
    bake_item_stack(
        env,
        stack,
        BlockModelDisplayContext::Ground,
        &entity_item_owner(env.world, instance, using_item),
        &ItemPlacement::attached_to_entity(
            instance,
            transform,
            false,
            BLOCK_GROUND_FALLBACK,
            GENERATED_GROUND_FALLBACK,
        ),
        residency,
    );
}

/// Bakes the non-skull, non-armor item rendered by vanilla `CustomHeadLayer` from the HEAD equipment
/// slot. Skull block items use `SkullBlockRenderer` in vanilla and stay deferred to the dedicated skull
/// path; humanoid armor with an equipment asset is already emitted by the armor overlay.
pub(super) fn bake_custom_head_item(
    instance: &EntityModelInstance,
    env: &ItemBakeEnv<'_>,
    residency: &mut ItemModelResidencyFrame,
) {
    let Some(stack) = env
        .world
        .equipment_item(instance.entity_id, EquipmentSlot::Head)
    else {
        return;
    };
    let Some(item_id) = stack.item_id else {
        return;
    };
    if instance.render_state.head_equipment_asset.is_some() {
        return;
    }
    if env
        .item_runtime
        .item_resource_id(item_id)
        .is_some_and(bbb_client_world::block_name_is_floor_skull)
    {
        return;
    }
    for transform in custom_head_item_transforms(instance) {
        bake_item_stack(
            env,
            stack,
            BlockModelDisplayContext::Head,
            &entity_item_owner(env.world, instance, false),
            &ItemPlacement::attached_to_entity(
                instance,
                transform,
                false,
                BLOCK_HEAD_FALLBACK,
                GENERATED_HEAD_FALLBACK,
            ),
            residency,
        );
    }
}

fn entity_item_owner(
    world: &WorldStore,
    instance: &EntityModelInstance,
    using_item: bool,
) -> ItemOwner<'static> {
    ItemOwner {
        entity_type: entity_model_context_entity_type(instance.kind),
        main_hand_left: Some(instance.render_state.main_arm_left),
        team_color_rgb: world.entity_team_color_rgb(instance.entity_id),
        using_item,
        using_item_ticks: entity_use_elapsed_ticks(instance, using_item),
    }
}

pub(super) fn entity_hand_using_item(instance: &EntityModelInstance, off_hand: bool) -> bool {
    instance.render_state.is_using_item && instance.render_state.use_item_off_hand == off_hand
}

/// Vanilla `AvatarRenderer.extractRenderState`'s `useItem.is(Items.SPYGLASS)`: the held-to-eye
/// branch keys on the item id, not the use animation.
pub(super) fn held_stack_is_spyglass(
    item_runtime: &NativeItemRuntime,
    stack: &ItemStackSummary,
) -> bool {
    stack.item_id.is_some_and(|item_id| {
        item_runtime
            .item_resource_id(item_id)
            .is_some_and(|resource_id| {
                item_types::vanilla_item_resource_id_matches_type(
                    resource_id,
                    item_types::VANILLA_ITEM_SPYGLASS_ID,
                )
            })
    })
}

pub(super) fn entity_use_elapsed_ticks(instance: &EntityModelInstance, using_item: bool) -> f32 {
    if using_item {
        instance.render_state.crossbow_charge_ticks
    } else {
        0.0
    }
}

pub(in crate::item_models) fn entity_model_context_entity_type(
    kind: EntityModelKind,
) -> Option<&'static str> {
    match kind {
        EntityModelKind::Player { .. } => Some("minecraft:player"),
        EntityModelKind::ArmorStand { .. } => Some("minecraft:armor_stand"),
        EntityModelKind::Zombie { .. } => Some("minecraft:zombie"),
        EntityModelKind::ZombieVariant { family, .. } => match family {
            ZombieVariantModelFamily::Husk => Some("minecraft:husk"),
            ZombieVariantModelFamily::Drowned => Some("minecraft:drowned"),
            ZombieVariantModelFamily::ZombieVillager => Some("minecraft:zombie_villager"),
        },
        EntityModelKind::Skeleton => Some("minecraft:skeleton"),
        EntityModelKind::SkeletonVariant { family } => match family {
            SkeletonModelFamily::Stray => Some("minecraft:stray"),
            SkeletonModelFamily::Parched => Some("minecraft:parched"),
            SkeletonModelFamily::WitherSkeleton => Some("minecraft:wither_skeleton"),
            SkeletonModelFamily::Bogged { .. } => Some("minecraft:bogged"),
        },
        EntityModelKind::Piglin { family, .. } => match family {
            PiglinModelFamily::Piglin => Some("minecraft:piglin"),
            PiglinModelFamily::PiglinBrute => Some("minecraft:piglin_brute"),
            PiglinModelFamily::ZombifiedPiglin => Some("minecraft:zombified_piglin"),
        },
        EntityModelKind::Giant => Some("minecraft:giant"),
        EntityModelKind::Vex { .. } => Some("minecraft:vex"),
        EntityModelKind::Allay => Some("minecraft:allay"),
        EntityModelKind::Dolphin { .. } => Some("minecraft:dolphin"),
        EntityModelKind::Villager { .. } => Some("minecraft:villager"),
        EntityModelKind::WanderingTrader => Some("minecraft:wandering_trader"),
        EntityModelKind::Fox { .. } => Some("minecraft:fox"),
        EntityModelKind::Panda { .. } => Some("minecraft:panda"),
        EntityModelKind::CopperGolem { .. } => Some("minecraft:copper_golem"),
        EntityModelKind::Witch => Some("minecraft:witch"),
        EntityModelKind::Illager { family } => match family {
            IllagerModelFamily::Evoker => Some("minecraft:evoker"),
            IllagerModelFamily::Illusioner => Some("minecraft:illusioner"),
            IllagerModelFamily::Pillager => Some("minecraft:pillager"),
            IllagerModelFamily::Vindicator => Some("minecraft:vindicator"),
        },
        _ => None,
    }
}
