use super::dropped::{
    bake_item_stack_at_transform, world_attribute_keys, world_enchantment_keys,
    world_trim_material_keys,
};
use super::*;

/// Vanilla `DataComponents.KINETIC_WEAPON` network type id, used by item-stack
/// patches to remove a spear's prototype kinetic weapon component.
pub(super) const VANILLA_KINETIC_WEAPON_COMPONENT_ID: i32 = 39;
/// Vanilla `DataComponents.SWING_ANIMATION` network type id, used by item-stack
/// patches to remove prototype STAB/NONE data and fall back to
/// `SwingAnimation.DEFAULT` (WHACK, 6 ticks).
pub(super) const VANILLA_SWING_ANIMATION_COMPONENT_ID: i32 = 40;
/// Vanilla `DataComponents.CONSUMABLE` network type id. `Item.getUseAnimation`
/// checks this before `BLOCKS_ATTACKS`, so consumable stacks do not use the
/// BLOCK first-person transform even when a patch also adds `blocks_attacks`.
pub(super) const VANILLA_CONSUMABLE_COMPONENT_ID: i32 = 24;
/// Vanilla `DataComponents.BLOCKS_ATTACKS` network type id. Non-consumable
/// stacks carrying it use `ItemUseAnimation.BLOCK`; the vanilla shield has it as
/// a prototype component.
pub(super) const VANILLA_BLOCKS_ATTACKS_COMPONENT_ID: i32 = 37;
/// Vanilla `BrushItem.USE_DURATION`. The first-person BRUSH swipe uses the
/// remaining use ticks modulo `BrushItem.ANIMATION_DURATION` (10).
pub(super) const VANILLA_BRUSH_USE_DURATION_TICKS: f32 = 200.0;
/// Vanilla `BowItem.getUseDuration`.
pub(super) const VANILLA_BOW_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla `CrossbowItem.getUseDuration`.
pub(super) const VANILLA_CROSSBOW_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla base `CrossbowItem.getChargeDuration`: 1.25 seconds * 20 TPS.
pub(super) const VANILLA_CROSSBOW_CHARGE_DURATION_TICKS: f32 = 25.0;
/// Vanilla `TridentItem.getUseDuration`.
pub(super) const VANILLA_TRIDENT_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla base `Item.getUseDuration`: stacks with `DataComponents.KINETIC_WEAPON`
/// and no consumable use the long charged-use timer.
pub(super) const VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS: f32 = 72_000.0;
/// Vanilla `ItemInHandRenderer.renderMap`: after the hand/map pose, the map is flipped, scaled to
/// `0.38`, centered, and converted from 128x128 map pixels into model units.
pub(super) const VANILLA_FIRST_PERSON_MAP_SCALE: f32 = 0.38;
pub(super) const VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE: f32 = 1.0 / 128.0;

/// The baked first-person local-player hand item submissions for this frame. Ordinary items use
/// block/flat item-model meshes; filled maps use vanilla `ItemInHandRenderer.renderMap` dynamic-map
/// surfaces in the same depth-cleared hand pass.
pub(crate) struct FirstPersonItemModels {
    pub block_meshes: Vec<ItemModelMesh>,
    pub block_translucent_meshes: Vec<ItemModelMesh>,
    pub block_glint_meshes: Vec<ItemModelMesh>,
    pub block_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_meshes: Vec<ItemModelMesh>,
    pub flat_translucent_meshes: Vec<ItemModelMesh>,
    pub flat_glint_meshes: Vec<ItemModelMesh>,
    pub flat_glint_translucent_meshes: Vec<ItemModelMesh>,
    pub map_background_textures: Vec<FirstPersonMapBackgroundTexture>,
    pub map_background_surfaces: Vec<FirstPersonMapBackgroundSurface>,
    pub map_textures: Vec<ItemFrameMapTexture>,
    pub map_surfaces: Vec<ItemFrameMapSurface>,
    pub map_decoration_textures: Vec<ItemFrameMapDecorationTexture>,
    pub map_decoration_surfaces: Vec<ItemFrameMapDecorationSurface>,
    pub map_text_surfaces: Vec<ItemFrameMapTextSurface>,
}

impl FirstPersonItemModels {
    fn empty() -> Self {
        Self {
            block_meshes: Vec::new(),
            block_translucent_meshes: Vec::new(),
            block_glint_meshes: Vec::new(),
            block_glint_translucent_meshes: Vec::new(),
            flat_meshes: Vec::new(),
            flat_translucent_meshes: Vec::new(),
            flat_glint_meshes: Vec::new(),
            flat_glint_translucent_meshes: Vec::new(),
            map_background_textures: Vec::new(),
            map_background_surfaces: Vec::new(),
            map_textures: Vec::new(),
            map_surfaces: Vec::new(),
            map_decoration_textures: Vec::new(),
            map_decoration_surfaces: Vec::new(),
            map_text_surfaces: Vec::new(),
        }
    }
}

pub(crate) fn first_person_item_models(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    camera_pose: Option<CameraPose>,
    partial_ticks: f32,
) -> FirstPersonItemModels {
    let mut models = FirstPersonItemModels::empty();
    let (Some(item_runtime), Some(camera_pose)) = (item_runtime, camera_pose) else {
        return models;
    };
    if !world.local_player().camera.follows_player || world.local_player_is_spectator() {
        return models;
    }
    if local_player_is_scoping(world, item_runtime) {
        return models;
    }

    let main_stack = world.local_item_in_hand(InteractionHand::MainHand);
    let off_stack = world.local_item_in_hand(InteractionHand::OffHand);
    let using_hand = world.local_player().interaction.using_item.then(|| {
        world
            .local_player()
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand)
    });
    if let Some(hand) = using_hand {
        let stack = match hand {
            InteractionHand::MainHand => main_stack,
            InteractionHand::OffHand => off_stack,
        };
        if !stack.is_some_and(|stack| {
            first_person_stack_supported_use_animation(stack, item_runtime).is_some()
        }) {
            return models;
        }
    }
    if [main_stack, off_stack]
        .into_iter()
        .flatten()
        .any(|stack| !supported_first_person_item_stack(stack, item_runtime))
    {
        return models;
    }

    let enchantment_keys = world_enchantment_keys(world);
    let trim_material_keys = world_trim_material_keys(world);
    let attribute_keys = world_attribute_keys(world);
    let context_dimension = world.level_info().map(|level| level.dimension.as_str());
    let mut map_textures = BTreeMap::new();
    let owner_main_hand_left = world.local_player_main_arm_left().unwrap_or(false);
    let camera_x_rot = camera_pose.x_rot;
    let camera_world = first_person_camera_world_transform(camera_pose);
    let attack_swing = world.local_player_attack_swing(partial_ticks);
    let only_render_using_hand =
        first_person_only_render_using_hand(item_runtime, main_stack, off_stack, using_hand);

    for (hand, stack) in [
        (InteractionHand::MainHand, main_stack),
        (InteractionHand::OffHand, off_stack),
    ] {
        if only_render_using_hand.is_some_and(|only_hand| only_hand != hand) {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        let arm_left = match hand {
            InteractionHand::MainHand => owner_main_hand_left,
            InteractionHand::OffHand => !owner_main_hand_left,
        };
        let context = if arm_left {
            BlockModelDisplayContext::FirstPersonLeftHand
        } else {
            BlockModelDisplayContext::FirstPersonRightHand
        };
        let attack = attack_swing
            .filter(|swing| swing.off_hand == (hand == InteractionHand::OffHand))
            .map_or(0.0, |swing| swing.attack_anim);
        if let Some(map_id) = stack.component_patch.map_id {
            let map = world.map_item(map_id);
            let map_transform = if hand == InteractionHand::MainHand && off_stack.is_none() {
                first_person_two_handed_map_transform(camera_world, camera_x_rot, 0.0, attack)
            } else {
                first_person_one_handed_map_transform(camera_world, arm_left, 0.0, attack)
            };
            let background_kind = if map.is_some() {
                FirstPersonMapBackgroundKind::Checkerboard
            } else {
                FirstPersonMapBackgroundKind::Plain
            };
            models
                .map_background_surfaces
                .push(bake_first_person_map_background_surface(
                    background_kind,
                    map_transform,
                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                ));
            if let Some(map) = map {
                map_textures
                    .entry(map.id)
                    .or_insert_with(|| map_item_texture(map));
                models.map_surfaces.push(bake_item_frame_map_surface(
                    map.id,
                    map_transform,
                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                ));
                let mut visible_decoration_index = 0;
                let mut text_submit_sequence = 0;
                for decoration in &map.decorations {
                    if let Some(surface) = bake_first_person_map_decoration_surface(
                        decoration.type_id,
                        decoration.x,
                        decoration.y,
                        decoration.rot,
                        visible_decoration_index,
                        map_transform,
                        ITEM_MODEL_FULL_BRIGHT_LIGHT,
                        visible_decoration_index + 1,
                    ) {
                        models.map_decoration_surfaces.push(surface);
                        if let Some(name) = decoration.name.as_ref() {
                            if let Some(glyphs) = item_runtime.map_text_glyphs() {
                                if let Some(text_surface) = bake_first_person_map_text_surface(
                                    decoration.type_id,
                                    name.as_str(),
                                    decoration.x,
                                    decoration.y,
                                    visible_decoration_index,
                                    map_transform,
                                    ITEM_MODEL_FULL_BRIGHT_LIGHT,
                                    text_submit_sequence,
                                    glyphs,
                                ) {
                                    models.map_text_surfaces.push(text_surface);
                                    text_submit_sequence += 1;
                                }
                            }
                        }
                        visible_decoration_index += 1;
                    }
                }
            }
            continue;
        }
        let mut attach =
            first_person_item_arm_transform_from_camera_world(camera_world, arm_left, 0.0);
        let using_this_hand = using_hand == Some(hand);
        if let Some(use_animation) = using_this_hand
            .then(|| first_person_stack_supported_use_animation(stack, item_runtime))
            .flatten()
        {
            match use_animation {
                FirstPersonUseAnimation::None => {}
                FirstPersonUseAnimation::EatDrink { use_duration_ticks } => {
                    attach = first_person_apply_eat_drink_use_transform(
                        camera_world,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Block(kind) => {
                    if kind == FirstPersonBlockUseKind::NonShieldBlock {
                        attach = first_person_apply_block_use_transform(attach, arm_left);
                    }
                }
                FirstPersonUseAnimation::Brush { use_duration_ticks } => {
                    attach = first_person_apply_brush_use_transform(
                        attach,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Bundle => {
                    attach = first_person_apply_whack_swing(attach, arm_left, attack);
                }
                FirstPersonUseAnimation::Trident { use_duration_ticks } => {
                    attach = first_person_apply_trident_use_transform(
                        attach,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Bow { use_duration_ticks } => {
                    attach = first_person_apply_bow_use_transform(
                        attach,
                        arm_left,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                    );
                }
                FirstPersonUseAnimation::Crossbow {
                    use_duration_ticks,
                    charge_duration_ticks,
                } => {
                    if first_person_stack_is_charged_crossbow(stack, item_runtime) {
                        attach = first_person_apply_whack_swing(attach, arm_left, attack);
                        if hand == InteractionHand::MainHand && attack < 0.001 {
                            attach = first_person_apply_charged_crossbow_idle_transform(
                                attach, arm_left,
                            );
                        }
                    } else {
                        attach = first_person_apply_crossbow_use_transform(
                            attach,
                            arm_left,
                            use_duration_ticks,
                            charge_duration_ticks,
                            world.local_player().interaction.using_item_ticks as f32,
                            partial_ticks,
                        );
                    }
                }
                FirstPersonUseAnimation::Spear {
                    kinetic_weapon,
                    use_duration_ticks,
                } => {
                    attach = first_person_apply_spear_use_transform(
                        camera_world,
                        arm_left,
                        kinetic_weapon,
                        use_duration_ticks,
                        world.local_player().interaction.using_item_ticks as f32,
                        partial_ticks,
                        world.local_player_ticks_since_kinetic_hit_feedback(partial_ticks),
                    );
                }
            }
        } else {
            match first_person_stack_swing_animation(stack, item_runtime) {
                FirstPersonSwingAnimation::None => {}
                FirstPersonSwingAnimation::Whack => {
                    attach = first_person_apply_whack_swing(attach, arm_left, attack);
                }
                FirstPersonSwingAnimation::Stab => {
                    attach = first_person_apply_stab_swing(attach, arm_left, attack);
                }
            }
            if first_person_stack_is_charged_crossbow(stack, item_runtime)
                && hand == InteractionHand::MainHand
                && attack < 0.001
            {
                attach = first_person_apply_charged_crossbow_idle_transform(attach, arm_left);
            }
        }
        bake_item_stack_at_transform(
            stack,
            attach,
            context,
            arm_left,
            Some(owner_main_hand_left),
            Some("minecraft:player"),
            using_this_hand,
            world.local_player().interaction.using_item_ticks as f32,
            enchantment_keys.as_deref(),
            trim_material_keys.as_deref(),
            attribute_keys.as_deref(),
            context_dimension,
            first_person_block_fallback(context),
            GENERATED_FIRST_PERSON_FALLBACK,
            item_runtime,
            terrain_textures,
            &mut models.block_meshes,
            &mut models.block_translucent_meshes,
            &mut models.block_glint_meshes,
            &mut models.block_glint_translucent_meshes,
            &mut models.flat_meshes,
            &mut models.flat_translucent_meshes,
            &mut models.flat_glint_meshes,
            &mut models.flat_glint_translucent_meshes,
        );
    }

    if !models.map_background_surfaces.is_empty() {
        models.map_background_textures = item_runtime.map_background_textures().to_vec();
    }
    models.map_textures = map_textures.into_values().collect();
    if !models.map_decoration_surfaces.is_empty() {
        models.map_decoration_textures = item_runtime.map_decoration_textures().to_vec();
    }
    models
}

pub(crate) fn first_person_player_arms(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    local_player_instance: Option<&EntityModelInstance>,
    camera_pose: Option<CameraPose>,
    partial_ticks: f32,
) -> Vec<FirstPersonPlayerArm> {
    let (Some(local_player_instance), Some(camera_pose)) = (local_player_instance, camera_pose)
    else {
        return Vec::new();
    };
    if !world.local_player().camera.follows_player || world.local_player_is_spectator() {
        return Vec::new();
    }
    if item_runtime.is_some_and(|item_runtime| local_player_is_scoping(world, item_runtime)) {
        return Vec::new();
    }
    if local_player_instance.render_state.invisible {
        return Vec::new();
    }
    let EntityModelKind::Player { skin, parts } = local_player_instance.kind else {
        return Vec::new();
    };

    let main_stack = world.local_item_in_hand(InteractionHand::MainHand);
    let off_stack = world.local_item_in_hand(InteractionHand::OffHand);
    let using_hand = world.local_player().interaction.using_item.then(|| {
        world
            .local_player()
            .interaction
            .using_item_hand
            .unwrap_or(InteractionHand::MainHand)
    });
    let only_render_using_hand = item_runtime.and_then(|item_runtime| {
        first_person_only_render_using_hand(item_runtime, main_stack, off_stack, using_hand)
    });
    let owner_main_hand_left = world.local_player_main_arm_left().unwrap_or(false);
    let camera_world = first_person_camera_world_transform(camera_pose);
    let attack_swing = world.local_player_attack_swing(partial_ticks);
    let hand_attack = |hand: InteractionHand| {
        attack_swing
            .filter(|swing| swing.off_hand == (hand == InteractionHand::OffHand))
            .map_or(0.0, |swing| swing.attack_anim)
    };
    let light = first_person_player_arm_light(local_player_instance);
    let mut arms = Vec::new();

    if only_render_using_hand.is_none_or(|hand| hand == InteractionHand::MainHand)
        && main_stack.is_none()
    {
        let arm_left = owner_main_hand_left;
        let attack = hand_attack(InteractionHand::MainHand);
        arms.push(FirstPersonPlayerArm {
            left: arm_left,
            skin,
            sleeve_visible: if arm_left {
                parts.left_sleeve
            } else {
                parts.right_sleeve
            },
            transform: first_person_render_player_arm_transform(
                camera_world,
                arm_left,
                0.0,
                attack,
            ),
            light,
        });
    }

    for (hand, stack) in [
        (InteractionHand::MainHand, main_stack),
        (InteractionHand::OffHand, off_stack),
    ] {
        if only_render_using_hand.is_some_and(|only_hand| only_hand != hand) {
            continue;
        }
        let Some(stack) = stack else {
            continue;
        };
        if stack.component_patch.map_id.is_none() {
            continue;
        }
        let attack = hand_attack(hand);
        if hand == InteractionHand::MainHand && off_stack.is_none() {
            for arm_left in [false, true] {
                arms.push(FirstPersonPlayerArm {
                    left: arm_left,
                    skin,
                    sleeve_visible: if arm_left {
                        parts.left_sleeve
                    } else {
                        parts.right_sleeve
                    },
                    transform: first_person_two_handed_map_arm_transform(
                        camera_world,
                        camera_pose.x_rot,
                        0.0,
                        attack,
                        arm_left,
                    ),
                    light,
                });
            }
        } else {
            let arm_left = match hand {
                InteractionHand::MainHand => owner_main_hand_left,
                InteractionHand::OffHand => !owner_main_hand_left,
            };
            arms.push(FirstPersonPlayerArm {
                left: arm_left,
                skin,
                sleeve_visible: if arm_left {
                    parts.left_sleeve
                } else {
                    parts.right_sleeve
                },
                transform: first_person_one_handed_map_arm_transform(
                    camera_world,
                    arm_left,
                    0.0,
                    attack,
                ),
                light,
            });
        }
    }

    arms
}

fn first_person_only_render_using_hand(
    item_runtime: &NativeItemRuntime,
    main_stack: Option<&ItemStackSummary>,
    off_stack: Option<&ItemStackSummary>,
    using_hand: Option<InteractionHand>,
) -> Option<InteractionHand> {
    using_hand.filter(|hand| {
        let stack = match hand {
            InteractionHand::MainHand => main_stack,
            InteractionHand::OffHand => off_stack,
        };
        stack
            .and_then(|stack| first_person_stack_resource_id(stack, item_runtime))
            .is_some_and(|resource_id| {
                matches!(resource_id, "minecraft:bow" | "minecraft:crossbow")
            })
    })
}

fn first_person_player_arm_light(instance: &EntityModelInstance) -> [f32; 2] {
    let block = (instance.render_state.light_coords >> 4) & 0xF;
    let sky = (instance.render_state.light_coords >> 20) & 0xF;
    [block as f32 / 15.0, sky as f32 / 15.0]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstPersonSwingAnimation {
    None,
    Whack,
    Stab,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FirstPersonBlockUseKind {
    Shield,
    NonShieldBlock,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FirstPersonUseAnimation {
    None,
    EatDrink {
        use_duration_ticks: f32,
    },
    Block(FirstPersonBlockUseKind),
    Brush {
        use_duration_ticks: f32,
    },
    Bundle,
    Trident {
        use_duration_ticks: f32,
    },
    Bow {
        use_duration_ticks: f32,
    },
    Crossbow {
        use_duration_ticks: f32,
        charge_duration_ticks: f32,
    },
    Spear {
        kinetic_weapon: SpearKineticWeapon,
        use_duration_ticks: f32,
    },
}

fn supported_first_person_item_stack(
    stack: &ItemStackSummary,
    _item_runtime: &NativeItemRuntime,
) -> bool {
    stack.item_id.is_some()
}

pub(super) fn first_person_stack_block_use_kind(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<FirstPersonBlockUseKind> {
    let Some(item_id) = stack.item_id else {
        return None;
    };
    if first_person_stack_has_added_component(stack, VANILLA_CONSUMABLE_COMPONENT_ID) {
        return None;
    }
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_BLOCKS_ATTACKS_COMPONENT_ID)
    {
        return None;
    }
    let is_shield = item_runtime.item_resource_id(item_id) == Some("minecraft:shield");
    if is_shield {
        return Some(FirstPersonBlockUseKind::Shield);
    }
    first_person_stack_has_added_component(stack, VANILLA_BLOCKS_ATTACKS_COMPONENT_ID)
        .then_some(FirstPersonBlockUseKind::NonShieldBlock)
}

pub(super) fn first_person_stack_supported_use_animation(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<FirstPersonUseAnimation> {
    match first_person_stack_resource_id(stack, item_runtime) {
        Some("minecraft:goat_horn") => return Some(FirstPersonUseAnimation::None),
        Some("minecraft:brush") => {
            return Some(FirstPersonUseAnimation::Brush {
                use_duration_ticks: VANILLA_BRUSH_USE_DURATION_TICKS,
            });
        }
        Some("minecraft:bundle") => return Some(FirstPersonUseAnimation::Bundle),
        Some("minecraft:trident") => {
            return Some(FirstPersonUseAnimation::Trident {
                use_duration_ticks: VANILLA_TRIDENT_USE_DURATION_TICKS,
            });
        }
        Some("minecraft:bow") => {
            return Some(FirstPersonUseAnimation::Bow {
                use_duration_ticks: VANILLA_BOW_USE_DURATION_TICKS,
            });
        }
        Some("minecraft:crossbow") => {
            return Some(FirstPersonUseAnimation::Crossbow {
                use_duration_ticks: VANILLA_CROSSBOW_USE_DURATION_TICKS,
                charge_duration_ticks: VANILLA_CROSSBOW_CHARGE_DURATION_TICKS,
            });
        }
        _ => {}
    }
    if let Some(consumable) = first_person_stack_consumable_summary(stack, item_runtime) {
        let use_duration_ticks = first_person_consumable_use_duration_ticks(consumable);
        return match consumable.animation {
            ItemUseAnimationSummary::None => Some(FirstPersonUseAnimation::None),
            ItemUseAnimationSummary::Eat | ItemUseAnimationSummary::Drink => {
                Some(FirstPersonUseAnimation::EatDrink { use_duration_ticks })
            }
            ItemUseAnimationSummary::Block => Some(FirstPersonUseAnimation::Block(
                first_person_stack_block_kind_for_animation(stack, item_runtime)?,
            )),
            ItemUseAnimationSummary::Bow => {
                Some(FirstPersonUseAnimation::Bow { use_duration_ticks })
            }
            ItemUseAnimationSummary::Trident => {
                Some(FirstPersonUseAnimation::Trident { use_duration_ticks })
            }
            ItemUseAnimationSummary::Crossbow
            | ItemUseAnimationSummary::Spyglass
            | ItemUseAnimationSummary::TootHorn => Some(FirstPersonUseAnimation::None),
            ItemUseAnimationSummary::Brush => {
                Some(FirstPersonUseAnimation::Brush { use_duration_ticks })
            }
            ItemUseAnimationSummary::Bundle => Some(FirstPersonUseAnimation::Bundle),
            ItemUseAnimationSummary::Spear => Some(FirstPersonUseAnimation::Spear {
                kinetic_weapon: first_person_stack_spear_kinetic_weapon(stack, item_runtime)?,
                use_duration_ticks,
            }),
        };
    }
    if let Some(kinetic_weapon) = first_person_stack_spear_kinetic_weapon(stack, item_runtime) {
        return Some(FirstPersonUseAnimation::Spear {
            kinetic_weapon,
            use_duration_ticks: VANILLA_KINETIC_WEAPON_USE_DURATION_TICKS,
        });
    }
    first_person_stack_block_use_kind(stack, item_runtime).map(FirstPersonUseAnimation::Block)
}

fn first_person_stack_resource_id<'a>(
    stack: &ItemStackSummary,
    item_runtime: &'a NativeItemRuntime,
) -> Option<&'a str> {
    stack
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
}

fn first_person_stack_is_charged_crossbow(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> bool {
    first_person_stack_resource_id(stack, item_runtime) == Some("minecraft:crossbow")
        && !stack.component_patch.charged_projectiles_items.is_empty()
}

fn first_person_stack_consumable_summary(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<ConsumableSummary> {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_CONSUMABLE_COMPONENT_ID)
    {
        return None;
    }
    stack.component_patch.consumable.or_else(|| {
        first_person_stack_has_added_component(stack, VANILLA_CONSUMABLE_COMPONENT_ID)
            .then(ConsumableSummary::default)
            .or_else(|| {
                stack
                    .item_id
                    .and_then(|item_id| item_runtime.item_default_consumable(item_id))
            })
    })
}

fn first_person_stack_block_kind_for_animation(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<FirstPersonBlockUseKind> {
    let item_id = stack.item_id?;
    if item_runtime.item_resource_id(item_id) == Some("minecraft:shield") {
        Some(FirstPersonBlockUseKind::Shield)
    } else {
        Some(FirstPersonBlockUseKind::NonShieldBlock)
    }
}

fn first_person_consumable_use_duration_ticks(consumable: ConsumableSummary) -> f32 {
    (consumable.consume_seconds * 20.0) as i32 as f32
}

fn first_person_stack_has_added_component(stack: &ItemStackSummary, component_id: i32) -> bool {
    stack.component_patch.added_type_ids.contains(&component_id)
        && !stack
            .component_patch
            .removed_type_ids
            .contains(&component_id)
}

pub(super) fn first_person_stack_swing_animation(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> FirstPersonSwingAnimation {
    if first_person_stack_removes_swing_animation(stack) {
        return FirstPersonSwingAnimation::Whack;
    }
    if let Some(swing_animation) = stack.component_patch.swing_animation {
        return match swing_animation.animation_type {
            SwingAnimationTypeSummary::None => FirstPersonSwingAnimation::None,
            SwingAnimationTypeSummary::Whack => FirstPersonSwingAnimation::Whack,
            SwingAnimationTypeSummary::Stab => FirstPersonSwingAnimation::Stab,
        };
    }
    stack
        .item_id
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        .filter(|resource_id| first_person_resource_id_is_spear(resource_id))
        .map_or(FirstPersonSwingAnimation::Whack, |_| {
            FirstPersonSwingAnimation::Stab
        })
}

fn first_person_stack_spear_kinetic_weapon(
    stack: &ItemStackSummary,
    item_runtime: &NativeItemRuntime,
) -> Option<SpearKineticWeapon> {
    if stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_KINETIC_WEAPON_COMPONENT_ID)
    {
        return None;
    }
    let resource_id = first_person_stack_resource_id(stack, item_runtime)?;
    default_spear_kinetic_weapon_for_resource_id(resource_id)
}

fn first_person_resource_id_is_spear(resource_id: &str) -> bool {
    matches!(
        resource_id,
        "minecraft:wooden_spear"
            | "minecraft:stone_spear"
            | "minecraft:copper_spear"
            | "minecraft:iron_spear"
            | "minecraft:golden_spear"
            | "minecraft:diamond_spear"
            | "minecraft:netherite_spear"
    )
}

fn first_person_stack_removes_swing_animation(stack: &ItemStackSummary) -> bool {
    stack
        .component_patch
        .removed_type_ids
        .contains(&VANILLA_SWING_ANIMATION_COMPONENT_ID)
}

fn local_player_is_scoping(world: &WorldStore, item_runtime: &NativeItemRuntime) -> bool {
    world
        .local_using_item_item_id()
        .and_then(|item_id| item_runtime.item_resource_id(item_id))
        == Some("minecraft:spyglass")
}

fn first_person_block_fallback(context: BlockModelDisplayContext) -> BlockModelDisplayTransform {
    match context {
        BlockModelDisplayContext::FirstPersonLeftHand => BLOCK_FIRST_PERSON_LEFT_FALLBACK,
        BlockModelDisplayContext::FirstPersonRightHand => BLOCK_FIRST_PERSON_RIGHT_FALLBACK,
        _ => BLOCK_FIRST_PERSON_RIGHT_FALLBACK,
    }
}

pub(super) fn first_person_item_arm_transform_from_camera_world(
    camera_world: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    camera_world
        * Mat4::from_translation(Vec3::new(
            invert * 0.56,
            -0.52 + inverse_arm_height * -0.6,
            -0.72,
        ))
}

pub(super) fn first_person_one_handed_map_transform(
    camera_world: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    let attack = attack.clamp(0.0, 1.0);
    let sqrt_attack = attack.sqrt();
    let x_swing = (sqrt_attack * std::f32::consts::PI).sin();
    let x_swing_position = -0.5 * x_swing;
    let y_swing_position = 0.4 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.3 * (attack * std::f32::consts::PI).sin();
    let map_pose = camera_world
        * Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
        * Mat4::from_translation(Vec3::new(
            invert * 0.51,
            -0.08 + inverse_arm_height * -1.2,
            -0.75,
        ))
        * Mat4::from_translation(Vec3::new(
            invert * x_swing_position,
            y_swing_position - 0.3 * x_swing,
            z_swing_position,
        ))
        * Mat4::from_rotation_x((x_swing * -45.0).to_radians())
        * Mat4::from_rotation_y((invert * x_swing * -30.0).to_radians());
    first_person_apply_map_render_transform(map_pose)
}

pub(super) fn first_person_two_handed_map_transform(
    camera_world: Mat4,
    x_rot: f32,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let sqrt_attack = attack.sqrt();
    let y_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let z_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let map_tilt = first_person_map_tilt(x_rot);
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
    let map_pose = camera_world
        * Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
            -0.72,
        ))
        * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * 20.0).to_radians())
        * Mat4::from_scale(Vec3::splat(2.0));
    first_person_apply_map_render_transform(map_pose)
}

pub(super) fn first_person_render_player_arm_transform(
    parent_transform: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let sqrt_attack = attack.sqrt();
    let x_swing_position = -0.3 * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position = 0.4 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.4 * (attack * std::f32::consts::PI).sin();
    let z_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
    let y_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();

    parent_transform
        * Mat4::from_translation(Vec3::new(
            invert * (x_swing_position + 0.64000005),
            y_swing_position - 0.6 + inverse_arm_height * -0.6,
            z_swing_position - 0.71999997,
        ))
        * Mat4::from_rotation_y((invert * 45.0_f32).to_radians())
        * Mat4::from_rotation_y((invert * y_swing_rotation * 70.0).to_radians())
        * Mat4::from_rotation_z((invert * z_swing_rotation * -20.0).to_radians())
        * Mat4::from_translation(Vec3::new(invert * -1.0, 3.6, 3.5))
        * Mat4::from_rotation_z((invert * 120.0_f32).to_radians())
        * Mat4::from_rotation_x(200.0_f32.to_radians())
        * Mat4::from_rotation_y((invert * -135.0_f32).to_radians())
        * Mat4::from_translation(Vec3::new(invert * 5.6, 0.0, 0.0))
}

pub(super) fn first_person_one_handed_map_arm_transform(
    camera_world: Mat4,
    arm_left: bool,
    inverse_arm_height: f32,
    attack: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    first_person_render_player_arm_transform(
        camera_world
            * Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
            * Mat4::from_rotation_z((invert * 10.0_f32).to_radians()),
        arm_left,
        inverse_arm_height,
        attack,
    )
}

pub(super) fn first_person_two_handed_map_arm_transform(
    camera_world: Mat4,
    x_rot: f32,
    inverse_arm_height: f32,
    attack: f32,
    arm_left: bool,
) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let sqrt_attack = attack.sqrt();
    let y_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let z_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let map_tilt = first_person_map_tilt(x_rot);
    let invert = if arm_left { -1.0 } else { 1.0 };

    camera_world
        * Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
            -0.72,
        ))
        * Mat4::from_rotation_x((map_tilt * -85.0).to_radians())
        * Mat4::from_rotation_y(90.0_f32.to_radians())
        * Mat4::from_rotation_y(92.0_f32.to_radians())
        * Mat4::from_rotation_x(45.0_f32.to_radians())
        * Mat4::from_rotation_z((invert * -41.0_f32).to_radians())
        * Mat4::from_translation(Vec3::new(invert * 0.3, -1.1, 0.45))
}

fn first_person_apply_map_render_transform(transform: Mat4) -> Mat4 {
    transform
        * Mat4::from_rotation_y(180.0_f32.to_radians())
        * Mat4::from_rotation_z(180.0_f32.to_radians())
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_SCALE))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(VANILLA_FIRST_PERSON_MAP_PIXEL_SCALE))
}

pub(super) fn first_person_map_tilt(x_rot: f32) -> f32 {
    let tilt = (1.0 - x_rot / 45.0 + 0.1).clamp(0.0, 1.0);
    -(tilt * std::f32::consts::PI).cos() * 0.5 + 0.5
}

pub(super) fn first_person_apply_whack_swing(transform: Mat4, arm_left: bool, attack: f32) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let sqrt_attack = attack.sqrt();
    let x_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position = 0.2 * (sqrt_attack * std::f32::consts::TAU).sin();
    let z_swing_position = -0.2 * (attack * std::f32::consts::PI).sin();
    let y_swing_rotation = (attack * attack * std::f32::consts::PI).sin();
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();

    transform
        * Mat4::from_translation(Vec3::new(
            invert * x_swing_position,
            y_swing_position,
            z_swing_position,
        ))
        * Mat4::from_rotation_y((invert * (45.0 + y_swing_rotation * -20.0)).to_radians())
        * Mat4::from_rotation_z((invert * xz_swing_rotation * -20.0).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * -80.0).to_radians())
        * Mat4::from_rotation_y((invert * -45.0).to_radians())
}

pub(super) fn first_person_apply_stab_swing(transform: Mat4, arm_left: bool, attack: f32) -> Mat4 {
    let attack = attack.clamp(0.0, 1.0);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let starting_amount = first_person_ease_in_out_sine(first_person_progress(attack, 0.0, 0.05));
    let middle_amount = first_person_ease_out_back(first_person_progress(attack, 0.05, 0.2));
    let ending_amount = first_person_ease_in_out_expo(first_person_progress(attack, 0.4, 1.0));

    transform
        * Mat4::from_translation(Vec3::new(
            invert * 0.1 * (starting_amount - middle_amount),
            -0.075 * (starting_amount - ending_amount),
            0.65 * (starting_amount - middle_amount),
        ))
        * Mat4::from_rotation_x((-70.0 * (starting_amount - ending_amount)).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.25 * (ending_amount - middle_amount)))
}

pub(super) fn first_person_apply_block_use_transform(transform: Mat4, arm_left: bool) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    transform
        * Mat4::from_translation(Vec3::new(invert * -0.14142136, 0.08, 0.14142136))
        * Mat4::from_rotation_x((-102.25_f32).to_radians())
        * Mat4::from_rotation_y((invert * 13.365_f32).to_radians())
        * Mat4::from_rotation_z((invert * 78.05_f32).to_radians())
}

pub(super) fn first_person_apply_eat_drink_use_transform(
    camera_world: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let eat_transform = first_person_eat_drink_use_transform(
        arm_left,
        use_duration_ticks,
        using_item_ticks,
        partial_ticks,
    );
    first_person_item_arm_transform_from_camera_world(camera_world * eat_transform, arm_left, 0.0)
}

fn first_person_eat_drink_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    if use_duration_ticks <= 0.0 {
        return Mat4::IDENTITY;
    }
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let curr_usage_time = remaining_ticks - partial_ticks + 1.0;
    let scaled_usage_time = curr_usage_time / use_duration_ticks;
    let extra_height_offset = if scaled_usage_time < 0.8 {
        (curr_usage_time / 4.0 * std::f32::consts::PI).cos().abs() * 0.1
    } else {
        0.0
    };
    let eat_jiggle = 1.0 - scaled_usage_time.powf(27.0);
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(0.0, extra_height_offset, 0.0))
        * Mat4::from_translation(Vec3::new(eat_jiggle * 0.6 * invert, eat_jiggle * -0.5, 0.0))
        * Mat4::from_rotation_y((invert * eat_jiggle * 90.0).to_radians())
        * Mat4::from_rotation_x((eat_jiggle * 10.0).to_radians())
        * Mat4::from_rotation_z((invert * eat_jiggle * 30.0).to_radians())
}

pub(super) fn first_person_apply_brush_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_brush_use_transform(
            arm_left,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_brush_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    if use_duration_ticks <= 0.0 {
        return Mat4::IDENTITY;
    }
    let brush_animation_remaining_ticks = (use_duration_ticks - using_item_ticks).max(0.0) % 10.0;
    let delta_since_last_update = brush_animation_remaining_ticks - partial_ticks + 1.0;
    let scaled_usage_time = 1.0 - delta_since_last_update / 10.0;
    let current_swipe_angle = -15.0 + 75.0 * (scaled_usage_time * 2.0 * std::f32::consts::PI).cos();

    if arm_left {
        Mat4::from_translation(Vec3::new(0.1, 0.83, 0.35))
            * Mat4::from_rotation_x((-80.0_f32).to_radians())
            * Mat4::from_rotation_y((-90.0_f32).to_radians())
            * Mat4::from_rotation_x(current_swipe_angle.to_radians())
            * Mat4::from_translation(Vec3::new(-0.3, 0.22, 0.35))
    } else {
        Mat4::from_translation(Vec3::new(-0.25, 0.22, 0.35))
            * Mat4::from_rotation_x((-80.0_f32).to_radians())
            * Mat4::from_rotation_y(90.0_f32.to_radians())
            * Mat4::from_rotation_x(current_swipe_angle.to_radians())
    }
}

pub(super) fn first_person_apply_trident_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_trident_use_transform(
            arm_left,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_trident_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let power = (time_held / 10.0).min(1.0);
    let shake = if power > 0.1 {
        ((time_held - 0.1) * 1.3).sin() * (power - 0.1)
    } else {
        0.0
    };
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(invert * -0.5, 0.7, 0.1))
        * Mat4::from_rotation_x((-55.0_f32).to_radians())
        * Mat4::from_rotation_y((invert * 35.3_f32).to_radians())
        * Mat4::from_rotation_z((invert * -9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.2))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-invert * 45.0_f32).to_radians())
}

pub(super) fn first_person_apply_bow_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_bow_use_transform(
            arm_left,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_bow_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let mut power = time_held / 20.0;
    power = (power * power + power * 2.0) / 3.0;
    if power > 1.0 {
        power = 1.0;
    }
    let shake = if power > 0.1 {
        ((time_held - 0.1) * 1.3).sin() * (power - 0.1)
    } else {
        0.0
    };
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(invert * -0.2785682, 0.18344387, 0.15731531))
        * Mat4::from_rotation_x((-13.935_f32).to_radians())
        * Mat4::from_rotation_y((invert * 35.3_f32).to_radians())
        * Mat4::from_rotation_z((invert * -9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-invert * 45.0_f32).to_radians())
}

pub(super) fn first_person_apply_crossbow_use_transform(
    transform: Mat4,
    arm_left: bool,
    use_duration_ticks: f32,
    charge_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    transform
        * first_person_crossbow_use_transform(
            arm_left,
            use_duration_ticks,
            charge_duration_ticks,
            using_item_ticks,
            partial_ticks,
        )
}

fn first_person_crossbow_use_transform(
    arm_left: bool,
    use_duration_ticks: f32,
    charge_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let mut power = time_held / charge_duration_ticks.max(1.0);
    if power > 1.0 {
        power = 1.0;
    }
    let shake = if power > 0.1 {
        ((time_held - 0.1) * 1.3).sin() * (power - 0.1)
    } else {
        0.0
    };
    let invert = if arm_left { -1.0 } else { 1.0 };

    Mat4::from_translation(Vec3::new(invert * -0.4785682, -0.094387, 0.05731531))
        * Mat4::from_rotation_x((-11.935_f32).to_radians())
        * Mat4::from_rotation_y((invert * 65.3_f32).to_radians())
        * Mat4::from_rotation_z((invert * -9.785_f32).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * 0.04))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * 0.2))
        * Mat4::from_rotation_y((-invert * 45.0_f32).to_radians())
}

pub(super) fn first_person_apply_spear_use_transform(
    camera_world: Mat4,
    arm_left: bool,
    kinetic_weapon: SpearKineticWeapon,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
    ticks_since_kinetic_hit_feedback: f32,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    camera_world
        * Mat4::from_translation(Vec3::new(invert * 0.56, -0.52, -0.72))
        * first_person_spear_use_transform(
            arm_left,
            kinetic_weapon,
            use_duration_ticks,
            using_item_ticks,
            partial_ticks,
            ticks_since_kinetic_hit_feedback,
        )
}

fn first_person_spear_use_transform(
    arm_left: bool,
    kinetic_weapon: SpearKineticWeapon,
    use_duration_ticks: f32,
    using_item_ticks: f32,
    partial_ticks: f32,
    ticks_since_kinetic_hit_feedback: f32,
) -> Mat4 {
    let remaining_ticks = use_duration_ticks - using_item_ticks;
    let time_held = use_duration_ticks - (remaining_ticks - partial_ticks + 1.0);
    let params = kinetic_weapon.use_params(time_held);
    let invert = if arm_left { -1.0 } else { 1.0 };
    let x_rotation_degrees = -65.0 * first_person_ease_in_out_back(params.raise_progress)
        - 35.0 * params.lower_progress
        + 100.0 * params.raise_back_progress
        - 0.5 * params.sway_scale_fast;
    let y_negative_axis_rotation_degrees = invert
        * (-90.0 * first_person_progress(params.raise_progress, 0.5, 0.55)
            + 90.0 * params.sway_progress
            + 2.0 * params.sway_scale_slow);

    Mat4::from_translation(Vec3::new(
        invert
            * (params.raise_progress * 0.15
                + params.raise_progress_end * -0.05
                + params.sway_progress * -0.1
                + params.sway_scale_slow * 0.005),
        params.raise_progress * -0.075
            + params.raise_progress_middle * 0.075
            + params.sway_scale_fast * 0.01,
        params.raise_progress_start * 0.05
            + params.raise_progress_end * -0.05
            + params.sway_scale_slow * 0.005,
    )) * first_person_rotate_around(
        Vec3::new(0.0, 0.1, 0.0),
        Mat4::from_rotation_x(x_rotation_degrees.to_radians()),
    ) * first_person_rotate_around(
        Vec3::new(invert * 0.15, 0.0, 0.0),
        Mat4::from_rotation_y((-y_negative_axis_rotation_degrees).to_radians()),
    ) * Mat4::from_translation(Vec3::new(
        0.0,
        -first_person_spear_kinetic_hit_feedback_amount(ticks_since_kinetic_hit_feedback),
        0.0,
    ))
}

pub(super) fn first_person_apply_charged_crossbow_idle_transform(
    transform: Mat4,
    arm_left: bool,
) -> Mat4 {
    let invert = if arm_left { -1.0 } else { 1.0 };
    transform
        * Mat4::from_translation(Vec3::new(invert * -0.641864, 0.0, 0.0))
        * Mat4::from_rotation_y((invert * 10.0_f32).to_radians())
}

pub(super) fn first_person_rotate_around(pivot: Vec3, rotation: Mat4) -> Mat4 {
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
}

pub(super) fn first_person_spear_kinetic_hit_feedback_amount(
    ticks_since_feedback_start: f32,
) -> f32 {
    0.4 * (first_person_ease_out_quart(first_person_progress(ticks_since_feedback_start, 1.0, 3.0))
        - first_person_ease_in_out_sine(first_person_progress(
            ticks_since_feedback_start,
            3.0,
            10.0,
        )))
}

pub(super) fn first_person_progress(value: f32, start: f32, end: f32) -> f32 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}

fn first_person_ease_in_out_sine(value: f32) -> f32 {
    -((std::f32::consts::PI * value).cos() - 1.0) / 2.0
}

fn first_person_ease_out_quart(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(4)
}

fn first_person_ease_out_back(value: f32) -> f32 {
    1.0 + 2.70158 * (value - 1.0).powi(3) + 1.70158 * (value - 1.0).powi(2)
}

pub(super) fn first_person_ease_in_out_back(value: f32) -> f32 {
    if value < 0.5 {
        4.0 * value * value * (7.189819 * value - 2.5949094) / 2.0
    } else {
        let delta = 2.0 * value - 2.0;
        (delta * delta * (3.5949094 * delta + 2.5949094) + 2.0) / 2.0
    }
}

fn first_person_ease_in_out_expo(value: f32) -> f32 {
    if value < 0.5 {
        if value == 0.0 {
            0.0
        } else {
            2.0_f32.powf(20.0 * value - 10.0) / 2.0
        }
    } else if value == 1.0 {
        1.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * value + 10.0)) / 2.0
    }
}

pub(super) fn first_person_camera_world_transform(pose: CameraPose) -> Mat4 {
    first_person_camera_view_matrix(pose).inverse()
}

pub(super) fn first_person_camera_view_matrix(pose: CameraPose) -> Mat4 {
    let eye = Vec3::from_array(pose.position) + Vec3::Y * pose.eye_height;
    let yaw = pose.y_rot.to_radians();
    let pitch = pose.x_rot.to_radians();
    let cos_pitch = pitch.cos();
    let forward =
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch).normalize_or_zero();
    let target = eye
        + if forward.length_squared() > 0.0 {
            forward
        } else {
            Vec3::Z
        };
    Mat4::look_at_rh(eye, target, Vec3::Y)
}
