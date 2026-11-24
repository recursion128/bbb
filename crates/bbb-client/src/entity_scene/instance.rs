use super::*;
use bbb_protocol::packets::SwingAnimationTypeSummary;

pub(super) fn entity_model_instance(
    mut source: EntityRenderSource,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    game_time: i64,
    entity_partial_tick: f32,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<EntityModelInstance> {
    let (mut kind, dynamic_texture) = if source.entity_type_id
        >= bbb_protocol::content_registry::ContentRegistry::entity_base_id()
    {
        let spec = world
            .content_registry()
            .entity_by_id(source.entity_type_id)?;
        let anim_state = source
            .data_values
            .iter()
            .rev()
            .find(|value| value.data_id == 16)
            .map(|value| match &value.value {
                bbb_protocol::packets::EntityDataValueKind::Byte(b) => (*b).max(0) as u8,
                bbb_protocol::packets::EntityDataValueKind::ModCustom(bytes) => {
                    bytes.first().copied().unwrap_or(0)
                }
                _ => 0,
            })
            .unwrap_or(0);
        let catalog_lookup = variant_registries.and_then(|r| r.rig_catalog());
        let builtin_catalog;
        let catalog = match catalog_lookup {
            Some(catalog) => catalog,
            None => {
                builtin_catalog = bbb_client_assets::EntityRigCatalog::builtin_default();
                &builtin_catalog
            }
        };
        if let Some((rig_id, rig)) = catalog.rig_for_entity(spec) {
            let dynamic_texture = rig
                .texture_asset_id
                .as_deref()
                .and_then(|asset_id| variant_registries?.ready_texture(asset_id));
            (
                EntityModelKind::DataDriven { rig_id, anim_state },
                dynamic_texture,
            )
        } else {
            (EntityModelKind::Zombie { baby: false }, None)
        }
    } else {
        let mut kind = entity_model_kind_with_time(
            source.entity_type_id,
            &source.data_values,
            source.age_ticks as f32 + entity_partial_tick,
            game_time,
        );
        let dynamic_texture = apply_ready_entity_variant_presentation(
            &mut kind,
            source.entity_type_id,
            &source.data_values,
            variant_registries,
        );
        (kind, dynamic_texture)
    };
    // Vanilla `Armadillo.shouldHideInShell()` = `getState().shouldHideInShell(inStateTicks)`: the
    // shell-ball swap is gated on the client `inStateTicks`, which `entity_model_kind` (data-only)
    // cannot see, so it falls back to the steady SCARED hide. Override it with the world-projected
    // `isHidingInShell`, which also covers the ROLLING/UNROLLING transition windows.
    if let EntityModelKind::Armadillo { rolled_up, .. } = &mut kind {
        *rolled_up = source.render.kind.armadillo().is_hiding_in_shell;
    }
    // Vanilla `CreakingRenderer.extractRenderState`: outside teardown the emissive eyes follow the
    // synced `isActive()` bit selected by `entity_model_kind`; during teardown they instead follow
    // the client-ticked `hasGlowingEyes()` flicker. That renderer also suppresses the generic death
    // tip-over and red damage overlay so the dedicated `CREAKING_DEATH` keyframe owns the collapse.
    let creaking_is_tearing_down =
        bbb_client_world::vanilla_entity_metadata_creaking_tearing_down_accessor_for_entity_type(
            source.entity_type_id,
        )
        .is_some_and(|(data_id, default)| entity_data_bool(&source.data_values, data_id, default));
    if creaking_is_tearing_down {
        if let EntityModelKind::Creaking { eyes_glowing } = &mut kind {
            *eyes_glowing = source.creaking_eyes_glowing;
        }
        source.render.death_time = 0.0;
        source.render.has_red_overlay = false;
    }
    let player_textures = apply_player_profile_textures(&mut kind, &source, world, item_runtime);
    let player_cape_texture = player_textures.and_then(|textures| textures.cape);
    let player_elytra_texture = player_textures.and_then(|textures| textures.elytra);
    let hand_stacks = [
        world.held_item(source.entity_id, false),
        world.held_item(source.entity_id, true),
    ];
    let hand_resource_ids =
        hand_stacks.map(|stack| item_runtime?.item_resource_id(stack?.item_id?));
    let hand_is_item = |off_hand: bool, type_id| {
        hand_resource_ids[usize::from(off_hand)].is_some_and(|resource_id| {
            item_types::vanilla_item_resource_id_matches_type(resource_id, type_id)
        })
    };
    let hand_spears = hand_stacks.map(|stack| {
        world
            .item_profiles()
            .item_stack_spear_kinetic_weapon(stack?)
    });
    let hand_non_empty =
        |off_hand: bool| hand_stacks[usize::from(off_hand)].is_some_and(item_stack_non_empty);
    let hand_blocks_attacks = |off_hand: bool| {
        hand_stacks[usize::from(off_hand)]
            .is_some_and(|stack| item_stack_blocks_attacks(world, stack))
    };
    let hand_holds_charged_crossbow = |off_hand| {
        hand_is_item(off_hand, item_types::VANILLA_ITEM_CROSSBOW_ID)
            && hand_stacks[usize::from(off_hand)]
                .is_some_and(|stack| !stack.component_patch.charged_projectiles_items.is_empty())
    };
    let hand_holds_special_use_item = |off_hand| {
        hand_is_item(off_hand, item_types::VANILLA_ITEM_BOW_ID)
            || hand_is_item(off_hand, item_types::VANILLA_ITEM_CROSSBOW_ID)
            || hand_is_item(off_hand, item_types::VANILLA_ITEM_TRIDENT_ID)
            || hand_blocks_attacks(off_hand)
            || hand_is_item(off_hand, item_types::VANILLA_ITEM_SPYGLASS_ID)
            || hand_is_item(off_hand, item_types::VANILLA_ITEM_GOAT_HORN_ID)
            || hand_is_item(off_hand, item_types::VANILLA_ITEM_BRUSH_ID)
            || hand_spears[usize::from(off_hand)].is_some()
    };
    // Only skeletons drive the `BOW_AND_ARROW` aim pose.
    let main_hand_holds_bow = matches!(
        kind,
        EntityModelKind::Skeleton | EntityModelKind::SkeletonVariant { .. }
    ) && hand_is_item(false, item_types::VANILLA_ITEM_BOW_ID);
    let is_player = matches!(kind, EntityModelKind::Player { .. });
    let main_hand_holds_spear = hand_spears[0].is_some();
    let off_hand_holds_spear = hand_spears[1].is_some();
    let [main_hand_swing, off_hand_swing] =
        hand_stacks.map(|stack| item_stack_swing_animation_type(world, stack));
    let main_hand_swing_is_stab_type = main_hand_swing == SwingAnimationTypeSummary::Stab;
    let off_hand_swing_is_stab_type = off_hand_swing == SwingAnimationTypeSummary::Stab;
    let player_main_hand_holds_spear = is_player && main_hand_holds_spear;
    let player_off_hand_holds_spear = is_player && off_hand_holds_spear;
    let is_zombie_family = matches!(
        kind,
        EntityModelKind::Zombie { .. } | EntityModelKind::ZombieVariant { .. }
    );
    let is_humanoid_mob_renderer = is_zombie_family
        || matches!(
            kind,
            EntityModelKind::Skeleton
                | EntityModelKind::SkeletonVariant { .. }
                | EntityModelKind::Piglin { .. }
        );
    let player_main_hand_holds_charged_crossbow = is_player && hand_holds_charged_crossbow(false);
    let player_off_hand_holds_charged_crossbow = is_player && hand_holds_charged_crossbow(true);
    // Vanilla `AvatarRenderer.getArmPose` resolves a non-swinging charged main-hand crossbow before the
    // use-item branch, then `getArmPose(avatar, arm)` forces any non-empty off-hand pose to ITEM because
    // `CROSSBOW_HOLD.isTwoHanded()`. `HumanoidModel.setupAnim` applies that off-hand ITEM first and the
    // main-hand `CROSSBOW_HOLD` last, so the crossbow hold is the visible pose even if the off hand holds or
    // uses another special-pose item.
    let player_main_hand_crossbow_hold_pose =
        is_player && !source.is_swinging && player_main_hand_holds_charged_crossbow;
    let main_hand_crossbow_hold_forces_offhand_item =
        player_main_hand_crossbow_hold_pose && hand_non_empty(true);
    let using_offhand_forced_to_item_by_main_hand_crossbow =
        source.render.use_item_off_hand && main_hand_crossbow_hold_forces_offhand_item;
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` selects the `attackArm` hand stack and
    // copies its `getSwingAnimation().type()` into the render state: a spear swing uses `STAB`, a patched
    // `NONE` stack keeps the shared body/anchor swing prologue but skips the WHACK/STAB arm attack.
    let attack_hand_swing = if source.render.attack_arm_off_hand {
        off_hand_swing
    } else {
        main_hand_swing
    };
    // Vanilla `AvatarRenderer.getArmPose` use-item `ItemUseAnimation.SPEAR`: while the using hand holds a
    // spear, `HumanoidModel.ArmPose.SPEAR` applies `SpearAnimations.thirdPersonHandUse`, and
    // `ItemInHandLayer` applies `thirdPersonUseItem` using the spear's default `KineticWeapon`.
    let player_using_spear = if is_player
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
    {
        hand_spears[usize::from(source.render.use_item_off_hand)]
    } else {
        None
    };
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `SPYGLASS`: while a player is using a spyglass
    // (`isUsingItem` + the using hand holds a spyglass), the holding arm raises it to the eye. Only
    // `PlayerModel` consumes the use-item poses, so resolve the using-hand item just for the player kind.
    let player_using_spyglass = matches!(kind, EntityModelKind::Player { .. })
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_is_item(
            source.render.use_item_off_hand,
            item_types::VANILLA_ITEM_SPYGLASS_ID,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `TOOT_HORN`: while a player is tooting a goat
    // horn (`isUsingItem` + the using hand holds a goat horn), the holding arm raises it to the mouth.
    let player_tooting_horn = matches!(kind, EntityModelKind::Player { .. })
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_is_item(
            source.render.use_item_off_hand,
            item_types::VANILLA_ITEM_GOAT_HORN_ID,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `BRUSH`: while a player is brushing
    // (`isUsingItem` + the using hand holds a brush), the holding arm lowers to the brushed block.
    let player_brushing = matches!(kind, EntityModelKind::Player { .. })
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_is_item(
            source.render.use_item_off_hand,
            item_types::VANILLA_ITEM_BRUSH_ID,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `BLOCK` (`poseBlockingArm`): while a player raises a
    // non-consumable `BLOCKS_ATTACKS` item (the vanilla shield or a datapack/patch-granted blocker), the
    // holding arm tucks the blocking item forward along the head look.
    let player_blocking = matches!(kind, EntityModelKind::Player { .. })
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_blocks_attacks(source.render.use_item_off_hand);
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `THROW_TRIDENT`: while a player charges a trident
    // throw (`isUsingItem` + the using hand holds a trident, whose `TridentItem.getUseAnimation()` is
    // `TRIDENT`), the holding arm raises the trident straight overhead. Same `poseRightArm`/`poseLeftArm`
    // case the drowned reaches via aggression, here driven by the player use-item path.
    let player_throwing_trident = matches!(kind, EntityModelKind::Player { .. })
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_is_item(
            source.render.use_item_off_hand,
            item_types::VANILLA_ITEM_TRIDENT_ID,
        );
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `BOW_AND_ARROW` use-item arm pose: while a player
    // draws a bow (`isUsingItem` + the using hand holds a bow, `BowItem.getUseAnimation() == BOW`), BOTH arms
    // raise along the head look. The pose is two-handed and `affectsOffhandPose`, so `poseRightArm` sets both
    // arms and the opposite arm's pose is skipped. The renderer mirrors the brace yaw when the using hand is
    // off hand.
    let player_drawing_bow = matches!(kind, EntityModelKind::Player { .. })
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_is_item(
            source.render.use_item_off_hand,
            item_types::VANILLA_ITEM_BOW_ID,
        );
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `CROSSBOW_CHARGE` use-item arm pose
    // (`AnimationUtils.animateCrossbowCharge`, the same one the pillager/piglin use): while a player draws an
    // UNCHARGED crossbow (`isUsingItem` + the using hand holds a crossbow,
    // `CrossbowItem.getUseAnimation() == CROSSBOW`), the holding arm braces and the opposite arm pulls the
    // string back over the draw ticks
    // (`crossbow_charge_ticks`, the shared `getTicksUsingItem` counter advanced off `isUsingItem` in the
    // world tick loop). A CHARGED crossbow is excluded (`getArmPose` returns `CROSSBOW_HOLD` first).
    let player_charging_crossbow = is_player
        && source.render.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && hand_is_item(
            source.render.use_item_off_hand,
            item_types::VANILLA_ITEM_CROSSBOW_ID,
        )
        && !(if source.render.use_item_off_hand {
            player_off_hand_holds_charged_crossbow
        } else {
            player_main_hand_holds_charged_crossbow
        });
    // Vanilla `HumanoidModel.setupAnim` dispatch (lines 245-257): when a hand uses an `affectsOffhandPose`
    // item (the two-handed draws — `BOW_AND_ARROW` / `THROW_TRIDENT` / `CROSSBOW_CHARGE` / `CROSSBOW_HOLD`,
    // plus `SPEAR`), the OPPOSITE arm's `poseArm` is SKIPPED, so the opposite hand gets NO pose. Compute
    // this before the held-spear / crossbow-hold / item fallback flags so those projections share the same
    // vanilla skip. `SPYGLASS`/`TOOT_HORN`/`BRUSH`/`BLOCK` are NOT `affectsOffhandPose`.
    let main_hand_use_affects_offhand = source.render.is_using_item
        && !source.render.use_item_off_hand
        && (hand_is_item(false, item_types::VANILLA_ITEM_BOW_ID)
            || hand_is_item(false, item_types::VANILLA_ITEM_TRIDENT_ID)
            || hand_is_item(false, item_types::VANILLA_ITEM_CROSSBOW_ID)
            || player_main_hand_holds_spear);
    let off_hand_use_affects_offhand = source.render.is_using_item
        && source.render.use_item_off_hand
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && (hand_is_item(true, item_types::VANILLA_ITEM_BOW_ID)
            || hand_is_item(true, item_types::VANILLA_ITEM_TRIDENT_ID)
            || hand_is_item(true, item_types::VANILLA_ITEM_CROSSBOW_ID)
            || player_off_hand_holds_spear);
    // Vanilla `AvatarRenderer.getArmPose` `SPEAR`: a player merely holding a spear (or swinging it with
    // STAB) still gets `ArmPose.SPEAR`, which runs `SpearAnimations.thirdPersonHandUse` with no kinetic
    // branch because `ticksUsingItem <= 0`. In the non-using right-handed dispatch, an off-hand SPEAR runs
    // first and suppresses the main-hand pose unless a charged main-hand crossbow has already forced the
    // off-hand pose to ITEM through `CROSSBOW_HOLD.isTwoHanded()`.
    let player_main_hand_stab_swing_pose =
        is_player && source.is_swinging && main_hand_swing_is_stab_type;
    let player_off_hand_stab_swing_pose =
        is_player && source.is_swinging && off_hand_swing_is_stab_type;
    let player_main_hand_spear_pose = is_player
        && (player_main_hand_holds_spear || player_main_hand_stab_swing_pose)
        && !(source.render.is_using_item && !source.render.use_item_off_hand)
        && !off_hand_use_affects_offhand
        && (source.render.is_using_item
            || !(player_off_hand_holds_spear || player_off_hand_stab_swing_pose));
    let player_off_hand_spear_pose = is_player
        && (player_off_hand_holds_spear || player_off_hand_stab_swing_pose)
        && !(source.render.is_using_item && source.render.use_item_off_hand)
        && !main_hand_crossbow_hold_forces_offhand_item
        && !main_hand_use_affects_offhand;
    // Vanilla `HumanoidMobRenderer.getArmPose`: non-player humanoid mobs return `SPEAR` for a held item
    // tagged `minecraft:spears` (and for STAB while swinging). `AbstractZombieRenderer` also checks the
    // opposite hand's STAB component, so a zombie-family mob holding a spear in either hand can mark both
    // arm poses as `SPEAR`; the later right-handed `HumanoidModel.setupAnim` dispatch decides which arm
    // actually gets posed first / suppresses the other.
    let humanoid_mob_main_hand_spear_pose = is_humanoid_mob_renderer
        && (main_hand_holds_spear
            || (source.is_swinging && main_hand_swing_is_stab_type)
            || (is_zombie_family && off_hand_swing_is_stab_type));
    let humanoid_mob_off_hand_spear_pose = is_humanoid_mob_renderer
        && (off_hand_holds_spear
            || (source.is_swinging && off_hand_swing_is_stab_type)
            || (is_zombie_family && main_hand_swing_is_stab_type));
    // Vanilla `AvatarRenderer.getArmPose` `CROSSBOW_HOLD` (`AnimationUtils.animateCrossbowHold`, the same one
    // the pillager levels): a player holding a CHARGED main-hand crossbow while not mid-swing (`!swinging &&
    // crossbow && isCharged`, checked before the use-item branch) levels the crossbow along the head look.
    // Because `CROSSBOW_HOLD.isTwoHanded()`, `AvatarRenderer.getArmPose(avatar, arm)` forces any non-empty
    // off-hand pose to ITEM before setupAnim runs, so an off-hand spear/use pose cannot suppress it.
    let player_crossbow_hold = player_main_hand_crossbow_hold_pose;
    // Vanilla's off-hand `CROSSBOW_HOLD` (`poseLeftArm`) is skipped when the main hand already has an
    // affecting pose (`BOW_AND_ARROW`, `THROW_TRIDENT`, `CROSSBOW_CHARGE`/`HOLD`, or `SPEAR`). Otherwise a
    // charged off-hand crossbow levels the mirrored hold pose after the main hand's non-affecting pose.
    let player_crossbow_hold_off_hand = is_player
        && !source.is_swinging
        && !main_hand_use_affects_offhand
        && !player_main_hand_holds_spear
        && !player_main_hand_holds_charged_crossbow
        && player_off_hand_holds_charged_crossbow;
    // Whether a hand is the using hand holding a SPECIAL-pose item (so it gets its dedicated pose, NOT the
    // `ITEM` fallback). Any OTHER used item (food/potion -> `EAT`/`DRINK`, or a plain tool) falls through to
    // `ITEM`, so a player eating/drinking still shows the lowered `ITEM` arm.
    let main_hand_use_is_special = source.render.is_using_item
        && !source.render.use_item_off_hand
        && hand_holds_special_use_item(false);
    let off_hand_use_is_special = source.render.is_using_item
        && source.render.use_item_off_hand
        && hand_holds_special_use_item(true);
    // Vanilla `AvatarRenderer.getArmPose` fallback `ITEM` arm pose (`HumanoidModel.poseRightArm` ITEM case):
    // a player holding a generic main-hand item lowers and halves the arm swing — the `ITEM` branch reached
    // after the special-pose checks fail. Fires whether or not using, EXCEPT when this hand is using a special
    // item (`main_hand_use_is_special` -> its own pose) or the OFF hand draws an `affectsOffhandPose` item
    // (`off_hand_use_affects_offhand` -> vanilla skips this arm's `poseArm`). Spears (-> `SPEAR`) and charged
    // crossbows (-> `CROSSBOW_HOLD`) are excluded so their dedicated poses win; a non-charged crossbow or a
    // held (not drawn) bow correctly falls through to `ITEM`. Only `PlayerModel` consumes it.
    let player_main_hand_item_pose = is_player
        && !main_hand_use_is_special
        && !off_hand_use_affects_offhand
        && (source.render.is_using_item || !player_off_hand_holds_spear)
        && hand_non_empty(false)
        && !player_main_hand_holds_spear
        && !player_main_hand_holds_charged_crossbow;
    // Vanilla `AvatarRenderer.getArmPose(_, OFF_HAND)` fallback `ITEM` arm pose, posed onto the OFF (left)
    // arm by `HumanoidModel.poseLeftArm`: a player holding a plain off-hand item (shield/totem/block/food)
    // lowers and halves that arm. Mirror of the main-hand gate: fires whether or not using, EXCEPT when the
    // OFF hand uses a special item (`off_hand_use_is_special`) or the MAIN hand draws an `affectsOffhandPose`
    // item (`main_hand_use_affects_offhand` -> vanilla skips `poseLeftArm`). Excludes off-hand spears and
    // charged crossbows unless the main hand has already forced the off-hand pose to ITEM via
    // `CROSSBOW_HOLD.isTwoHanded()`; that ITEM pose is applied before the visible main-hand crossbow hold.
    let player_off_hand_item_pose = if main_hand_crossbow_hold_forces_offhand_item {
        true
    } else {
        is_player
            && !off_hand_use_is_special
            && !main_hand_use_affects_offhand
            && hand_non_empty(true)
            && !player_off_hand_holds_spear
            && !player_off_hand_holds_charged_crossbow
    };
    // Vanilla `Pillager.getArmPose` checks `isHolding(Items.CROSSBOW)`, which scans both hands, before
    // falling back to aggressive `ATTACKING`.
    let pillager_holds_crossbow = matches!(
        kind,
        EntityModelKind::Illager {
            family: IllagerModelFamily::Pillager
        }
    ) && (hand_is_item(false, item_types::VANILLA_ITEM_CROSSBOW_ID)
        || hand_is_item(true, item_types::VANILLA_ITEM_CROSSBOW_ID));
    // Vanilla `IllagerModel.setupAnim` `ATTACKING` branch selects empty-handed `animateZombieArms` versus
    // armed `swingWeaponDown` from the rendered main-hand item state.
    let illager_main_hand_empty =
        matches!(kind, EntityModelKind::Illager { .. }) && !hand_non_empty(false);
    // Vanilla `DrownedRenderer.getArmPose`: a drowned in its main hand holding a trident while aggressive
    // (`getMainArm() == arm && isAggressive() && item.is(Items.TRIDENT)`) raises the trident overhead to
    // throw it. `isAggressive` is already projected (the drowned is in the zombie model family).
    let drowned_throw_trident = matches!(
        kind,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            ..
        }
    ) && source.render.is_aggressive
        && hand_is_item(false, item_types::VANILLA_ITEM_TRIDENT_ID);
    // Vanilla `WitchRenderer.extractRenderState`: `isHoldingItem` is any non-empty main hand and
    // `isHoldingPotion` is exactly `Items.POTION`. The former drives the witch model's nose hold pose; the
    // latter selects `WitchItemLayer`'s nose-attached potion branch.
    let witch_holding_item = matches!(kind, EntityModelKind::Witch) && hand_non_empty(false);
    let witch_holding_potion =
        witch_holding_item && hand_is_item(false, item_types::VANILLA_ITEM_POTION_ID);
    // Vanilla `CopperGolemModel.setupAnim`: either rendered hand item state selects the held-item arm
    // clamp before `ItemInHandLayer` reads the fully posed hand transform, including the projected
    // walk-with-item keyframe.
    let copper_golem_holding_item = matches!(kind, EntityModelKind::CopperGolem { .. })
        && (hand_non_empty(false) || hand_non_empty(true));
    let custom_head_skull = entity_custom_head_skull(world, item_runtime, source.entity_id);
    // Vanilla `Piglin.getArmPose` `ADMIRING_ITEM` (`PiglinAi.isLovedItem(getOffhandItem())`): a regular
    // piglin holding a piglin-loved item in its OFFHAND admires it (head tilts down, the off arm lifts the
    // item). Second-highest priority (below DANCING, above ATTACKING / CROSSBOW), so it suppresses those.
    // Only `Piglin.getArmPose` has this branch — the brute's is ATTACKING/DEFAULT only — so gate to the
    // regular piglin. Resolve the `minecraft:piglin_loved` item tag just for it.
    let piglin_admiring = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            ..
        }
    ) && !piglin_is_dancing(source.entity_type_id, &source.data_values)
        && hand_stacks[1]
            .and_then(|stack| stack.item_id)
            .is_some_and(|item_id| {
                world.registry_holder_is_in_tag("minecraft:item", PIGLIN_LOVED_ITEM_TAG, item_id)
            });
    // Vanilla `Piglin.getArmPose` `CROSSBOW_HOLD`: a regular piglin holding a charged crossbow, not
    // dancing (top priority), not admiring (an offhand loved item, higher priority), and not mid-draw
    // (`CROSSBOW_CHARGE`, whose projected use duration drives the pull-back pose). The higher-priority
    // `ATTACKING_WITH_MELEE_WEAPON`
    // needs a tool main-hand item, so a charged-crossbow hand excludes it.
    let piglin_crossbow_hold = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            ..
        }
    ) && hand_holds_charged_crossbow(false)
        && !piglin_is_charging_crossbow(source.entity_type_id, &source.data_values)
        && !piglin_is_dancing(source.entity_type_id, &source.data_values)
        && !piglin_admiring;
    // Vanilla `Piglin`/`PiglinBrute.getArmPose` `ATTACKING_WITH_MELEE_WEAPON`: a piglin or piglin brute
    // that is aggressive (`Mob.isAggressive()`) and holds a melee weapon (`isHoldingMeleeWeapon()`, a
    // main-hand item with the `tool` component), not dancing, and (for the regular piglin) not admiring an
    // offhand loved item (both higher priority). The brute has no dance/admire/crossbow poses, so
    // `piglin_admiring` is always false for it. The zombified piglin uses its renderer zombie-arm pose
    // instead of this weapon-raised pose, so it is excluded.
    let piglin_attacking_with_melee = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin | PiglinModelFamily::PiglinBrute,
            ..
        }
    ) && source.render.is_aggressive
        && hand_stacks[0].is_some_and(|stack| {
            stack
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)
        })
        && !piglin_is_dancing(source.entity_type_id, &source.data_values)
        && !piglin_admiring;
    // Vanilla `Piglin.getArmPose` `CROSSBOW_CHARGE`: a regular piglin drawing its crossbow
    // (`isChargingCrossbow()`, the synced `DATA_IS_CHARGING_CROSSBOW` boolean id 18). Vanilla checks only
    // the flag (no held-item gate), but it ranks below DANCING / ADMIRING / ATTACKING, so suppress it under
    // those (a charging crossbow hand can never hold a melee tool, so the attack gate is also item-exclusive
    // — the explicit `!` is defensive). The pull-back ticks come from the shared `crossbow_charge_ticks`.
    let piglin_crossbow_charge =
        matches!(
            kind,
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                ..
            }
        ) && piglin_is_charging_crossbow(source.entity_type_id, &source.data_values)
            && !piglin_is_dancing(source.entity_type_id, &source.data_values)
            && !piglin_admiring
            && !piglin_attacking_with_melee;
    let pillager_crossbow_charge =
        pillager_is_charging_crossbow(source.entity_type_id, &source.data_values);
    let use_item_enchantments = if source.render.is_using_item {
        hand_stacks[usize::from(source.render.use_item_off_hand)]
            .map(|stack| stack.component_patch.enchantments.as_slice())
            .unwrap_or_default()
    } else {
        &[]
    };
    let resolve_crossbow_charge_duration = || {
        item_runtime.map_or_else(
            || world.crossbow_charge_duration_ticks(use_item_enchantments),
            |runtime| {
                crate::item_models::crossbow_charge_duration_ticks(
                    runtime,
                    world,
                    use_item_enchantments,
                )
            },
        ) as f32
    };
    let uses_humanoid_render_state = matches!(
        kind,
        EntityModelKind::Player { .. }
            | EntityModelKind::Zombie { .. }
            | EntityModelKind::ZombieVariant { .. }
            | EntityModelKind::Skeleton
            | EntityModelKind::SkeletonVariant { .. }
            | EntityModelKind::Piglin { .. }
            | EntityModelKind::ArmorStand { .. }
            | EntityModelKind::Enderman
            | EntityModelKind::Giant
    );
    let crossbow_charge_duration_ticks = if uses_humanoid_render_state || pillager_crossbow_charge {
        resolve_crossbow_charge_duration()
    } else {
        25.0
    };
    // Vanilla `Goat.getRammingXHeadRot()`: the world-projected `lowerHeadTick` ram counter scaled by the
    // adult/baby max head pitch. Resolved here because the baby flag lives in the goat kind.
    let goat_ramming_x_head_rot = match kind {
        EntityModelKind::Goat { baby, .. } => {
            goat_ramming_x_head_rot(source.goat_lower_head_tick, baby)
        }
        _ => 0.0,
    };
    // Vanilla `Camel.setupAnimationStates()`: the sit/sit-pose/stand-up timing projected purely
    // from the synced `LAST_POSE_CHANGE_TICK` and the world game time.
    let camel_sit = camel_sit_state(source.entity_type_id, &source.data_values, game_time);
    // Vanilla LivingEntityRenderer.extractRenderState:
    //   state.yRot = Mth.wrapDegrees(headRot - bodyRot)  (net head-look yaw)
    //   state.xRot = entity.getXRot(partialTicks)         (head-look pitch)
    // The net head yaw is taken against the unshaken body yaw; the setupRotations
    // body shake (freezing or a per-renderer conversion) is then folded into the
    // projected body_rot so the whole model jitters while the head turn relative to
    // the body is unchanged.
    // Vanilla LivingEntityRenderer.extractRenderState negates the net head yaw and
    // pitch while the entity is upside down (Dinnerbone/Grumm).
    let head_sign = if source.render.upside_down_height.is_some() {
        -1.0
    } else {
        1.0
    };
    // Vanilla `Squid.aiStep` refines `yBodyRot` from movement independently of
    // the synced transform yaw, and `LivingEntityRenderer.extractRenderState`
    // uses that value as `bodyRot`. WorldStore projects the other entities'
    // independent body yaw through the render source.
    let projected_body_yaw = if is_squid_entity_type(source.entity_type_id) {
        source.squid_y_body_rot
    } else {
        source.y_rot
    };
    let net_head_yaw = wrap_degrees(source.y_head_rot - projected_body_yaw) * head_sign;
    let head_pitch = source.x_rot * head_sign;
    let is_shaking = entity_shaking(
        source.entity_type_id,
        &source.data_values,
        source.is_fully_frozen,
        || {
            let position = world
                .probe_entity_transform(source.entity_id)
                .map(|transform| {
                    [
                        transform.position.x,
                        transform.position.y,
                        transform.position.z,
                    ]
                })
                .unwrap_or([source.position.x, source.position.y, source.position.z]);
            world_piglins_zombify(world, position)
        },
    );
    let body_rot = projected_body_yaw + entity_body_shake_degrees(source.age_ticks, is_shaking);
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` fills Vex right/left item states
    // from `getItemHeldByArm(RIGHT/LEFT)`. Canonical main/off hands flip sides when `main_arm_left` is true.
    let is_vex = matches!(kind, EntityModelKind::Vex { .. });
    let vex_right_hand_item_non_empty = is_vex && hand_non_empty(source.render.main_arm_left);
    let vex_left_hand_item_non_empty = is_vex && hand_non_empty(!source.render.main_arm_left);
    // Vanilla setupRotations sleeping branch: the bed yaw (or the body yaw when not
    // in a bed) plus the bed head-offset translate.
    let sleeping = source.is_sleeping.then_some(SleepingPose {
        yaw_angle: source.sleeping_bed_yaw.unwrap_or(body_rot),
        bed_offset: source.sleeping_bed_offset,
    });
    let fishing_line_origin_offset = world
        .fishing_hook_render_source_at_partial_tick(source.entity_id, entity_partial_tick)
        .map(fishing_hook_third_person_line_origin_offset);
    // Frozen shake, auto-spin and sheep grazing already have their own pose inputs.
    let mesh_age_in_ticks = match &mut kind {
        EntityModelKind::Sheep {
            jeb: false,
            age_ticks,
            ..
        } => {
            // Only the named rainbow wool layer reads this clock.
            *age_ticks = 0.0;
            0.0
        }
        EntityModelKind::ArmorStand { .. }
        | EntityModelKind::Feline { .. }
        | EntityModelKind::Cow { .. } => 0.0,
        _ => source.age_ticks as f32 + entity_partial_tick,
    };
    let trim =
        |sprite: &Option<WorldArmorTrimSprite>| armor_trim(sprite.as_ref(), variant_registries);
    let trims = &source.armor_trims;
    let data_values = &source.data_values;
    let type_id = source.entity_type_id;
    let mut state = source.render;
    state.body_rot = body_rot;
    state.head_yaw = net_head_yaw;
    state.head_pitch = head_pitch;
    state.sleeping = sleeping;
    state.age_in_ticks = mesh_age_in_ticks;
    state.main_hand_holds_bow = main_hand_holds_bow;
    state.main_hand_swing_is_stab = attack_hand_swing == SwingAnimationTypeSummary::Stab;
    state.main_hand_swing_is_none = attack_hand_swing == SwingAnimationTypeSummary::None;
    state.humanoid_mob_main_hand_spear_pose = humanoid_mob_main_hand_spear_pose;
    state.humanoid_mob_off_hand_spear_pose = humanoid_mob_off_hand_spear_pose;
    state.head_armor_trim = trim(&trims.head_armor_trim);
    state.chest_armor_trim = trim(&trims.chest_armor_trim);
    state.legs_armor_trim = trim(&trims.legs_armor_trim);
    state.feet_armor_trim = trim(&trims.feet_armor_trim);
    state.chest_wings_trim = trim(&trims.chest_wings_trim);
    state.crossbow_charge_duration_ticks = crossbow_charge_duration_ticks;
    state.custom_head_skull = custom_head_skull;
    match &mut state.kind {
        EntityKindRenderState::Trident(trident) => {
            trident.foil = thrown_trident_foil(type_id, data_values);
        }
        EntityKindRenderState::FishingHook(hook) => {
            hook.line_origin_offset = fishing_line_origin_offset;
        }
        EntityKindRenderState::Player(player) => {
            player.using_spear = player_using_spear;
            player.main_hand_spear_pose = player_main_hand_spear_pose;
            player.off_hand_spear_pose = player_off_hand_spear_pose;
            player.using_spyglass = player_using_spyglass;
            player.tooting_horn = player_tooting_horn;
            player.brushing = player_brushing;
            player.blocking = player_blocking;
            player.throwing_trident = player_throwing_trident;
            player.drawing_bow = player_drawing_bow;
            player.charging_crossbow = player_charging_crossbow;
            player.crossbow_hold = player_crossbow_hold;
            player.crossbow_hold_off_hand = player_crossbow_hold_off_hand;
            player.main_hand_item_pose = player_main_hand_item_pose;
            player.off_hand_item_pose = player_off_hand_item_pose;
            player.cape_texture = player_cape_texture;
            player.elytra_texture = player_elytra_texture;
        }
        EntityKindRenderState::Wolf(wolf) => {
            wolf.body_armor_trim = trim(&trims.wolf_body_armor_trim);
            wolf.tail_angle = wolf_tail_angle(type_id, data_values, game_time);
            wolf.sitting = wolf_sitting(type_id, data_values);
        }
        EntityKindRenderState::Pig(pig) => pig.saddle_trim = trim(&trims.pig_saddle_trim),
        EntityKindRenderState::Equine(equine) => {
            equine.saddle_trim = trim(&trims.equine_saddle_trim);
            equine.body_armor_trim = trim(&trims.equine_body_armor_trim);
        }
        EntityKindRenderState::Strider(strider) => {
            strider.saddle_trim = trim(&trims.strider_saddle_trim);
        }
        EntityKindRenderState::Camel(camel) => {
            camel.saddle_trim = trim(&trims.camel_saddle_trim);
            camel.sit_seconds = camel_sit.sit_seconds;
            camel.sit_pose_seconds = camel_sit.sit_pose_seconds;
            camel.standup_seconds = camel_sit.standup_seconds;
        }
        EntityKindRenderState::Nautilus(nautilus) => {
            nautilus.saddle_trim = trim(&trims.nautilus_saddle_trim);
            nautilus.body_armor_trim = trim(&trims.nautilus_body_armor_trim);
        }
        EntityKindRenderState::HappyGhast(ghast) => {
            ghast.harness_trim = trim(&trims.happy_ghast_harness_trim);
        }
        EntityKindRenderState::Llama(llama) => {
            llama.body_decor_trim = trim(&trims.llama_body_decor_trim);
        }
        EntityKindRenderState::Illager(illager) => {
            illager.holds_crossbow = pillager_holds_crossbow;
            illager.main_hand_empty = illager_main_hand_empty;
            illager.is_charging_crossbow = pillager_crossbow_charge;
            illager.celebrating = illager_celebrating(type_id, data_values);
        }
        EntityKindRenderState::Drowned(drowned) => drowned.throw_trident = drowned_throw_trident,
        EntityKindRenderState::Witch(witch) => {
            witch.holding_item = witch_holding_item;
            witch.holding_potion = witch_holding_potion;
        }
        EntityKindRenderState::CopperGolem(golem) => golem.holding_item = copper_golem_holding_item,
        EntityKindRenderState::Bee(bee) => {
            bee.angry = bee_is_angry(type_id, data_values, game_time)
        }
        EntityKindRenderState::Vex(vex) => {
            vex.right_hand_item_non_empty = vex_right_hand_item_non_empty;
            vex.left_hand_item_non_empty = vex_left_hand_item_non_empty;
        }
        EntityKindRenderState::Wither(wither) => {
            wither.powered = wither_powered(type_id, data_values)
        }
        EntityKindRenderState::Parrot(parrot) => {
            parrot.sitting = parrot_sitting(type_id, data_values)
        }
        EntityKindRenderState::Piglin(piglin) => {
            piglin.dancing = piglin_is_dancing(type_id, data_values);
            piglin.crossbow_hold = piglin_crossbow_hold;
            piglin.crossbow_charge = piglin_crossbow_charge;
            piglin.attacking_with_melee = piglin_attacking_with_melee;
            piglin.admiring = piglin_admiring;
        }
        EntityKindRenderState::Panda(panda) => {
            panda.unhappy = panda_is_unhappy(type_id, data_values);
            panda.sneezing = panda_is_sneezing(type_id, data_values);
            panda.sneeze_time = panda_sneeze_time(type_id, data_values);
            panda.eating = panda_is_eating(type_id, data_values);
            panda.scared = panda_is_scared(type_id, data_values, world);
            panda.sitting = panda_is_sitting(type_id, data_values);
        }
        EntityKindRenderState::Goat(goat) => goat.ramming_x_head_rot = goat_ramming_x_head_rot,
        EntityKindRenderState::Turtle(turtle) => {
            turtle.has_egg = turtle_has_egg(type_id, data_values);
            turtle.laying_egg = turtle_laying_egg(type_id, data_values);
        }
        EntityKindRenderState::EndCrystal(crystal) => {
            crystal.shows_bottom = end_crystal_shows_bottom(type_id, data_values);
        }
        EntityKindRenderState::Creeper(creeper) => {
            creeper.powered = creeper_powered(type_id, data_values);
        }
        EntityKindRenderState::Villager(villager) => {
            villager.model_data = villager_model_data(type_id, data_values);
        }
        _ => {}
    }
    state.white_overlay_progress = creeper_white_overlay_progress(state.kind.creeper().swelling);
    Some(EntityModelInstance {
        entity_id: source.entity_id,
        kind,
        position: [
            source.position.x as f32,
            source.position.y as f32,
            source.position.z as f32,
        ],
        dynamic_texture,
        render_state: state,
        block_entity: BlockEntityRenderState::default(),
    })
}

pub(super) fn fishing_hook_third_person_line_origin_offset(
    source: FishingHookRenderSourceState,
) -> [f32; 3] {
    let arm = if source.holding_arm_left { -1.0 } else { 1.0 };
    let yaw = source.owner_body_y_rot.to_radians();
    let right_offset = arm * 0.35 * source.owner_scale;
    let forward_offset = 0.8 * source.owner_scale;
    let crouch_offset = if source.owner_is_crouching {
        -0.1875
    } else {
        0.0
    };
    let hand = [
        source.owner_eye_position.x as f32 - yaw.cos() * right_offset - yaw.sin() * forward_offset,
        source.owner_eye_position.y as f32 + crouch_offset - 0.45 * source.owner_scale,
        source.owner_eye_position.z as f32 - yaw.sin() * right_offset + yaw.cos() * forward_offset,
    ];
    fishing_hook_line_origin_offset(source, hand)
}

/// Vanilla `FishingHookRenderer.extractRenderState` first-person local-player
/// hand point, including the attack-swing rotations around the current camera near plane.
pub(super) fn fishing_hook_first_person_line_origin_offset(
    source: FishingHookRenderSourceState,
    camera_pose: CameraPose,
    surface_size: [u32; 2],
    base_camera_fov_degrees: f32,
) -> [f32; 3] {
    let arm = if source.holding_arm_left { -1.0 } else { 1.0 };
    let swing = (source.owner_attack_anim.sqrt() * std::f32::consts::PI).sin();
    let yaw = camera_pose.y_rot.to_radians();
    let pitch = camera_pose.x_rot.to_radians();
    let forward = [
        -yaw.sin() * pitch.cos(),
        -pitch.sin(),
        yaw.cos() * pitch.cos(),
    ];
    let right_len = (forward[0] * forward[0] + forward[2] * forward[2]).sqrt();
    let right = if right_len > f32::EPSILON {
        [-forward[2] / right_len, 0.0, forward[0] / right_len]
    } else {
        [1.0, 0.0, 0.0]
    };
    let up = [
        right[1] * forward[2] - right[2] * forward[1],
        right[2] * forward[0] - right[0] * forward[2],
        right[0] * forward[1] - right[1] * forward[0],
    ];
    let aspect = surface_size[0] as f32 / surface_size[1].max(1) as f32;
    let options_fov = base_camera_fov_degrees;
    let near_height = (options_fov.to_radians() * 0.5).tan() * 0.05;
    let near_width = near_height * aspect;
    let x = arm * 0.525 * near_width;
    let y = -0.1 * near_height;
    let mut hand_offset = [
        forward[0] * 0.05 + right[0] * x + up[0] * y,
        forward[1] * 0.05 + right[1] * x + up[1] * y,
        forward[2] * 0.05 + right[2] * x + up[2] * y,
    ];
    let view_bobbing_scale = 960.0 / options_fov;
    for component in &mut hand_offset {
        *component *= view_bobbing_scale;
    }
    hand_offset = rotate_y(hand_offset, swing * 0.5);
    hand_offset = rotate_x(hand_offset, swing * 0.7);
    let hand = [
        source.owner_eye_position.x as f32 + hand_offset[0],
        source.owner_eye_position.y as f32 + hand_offset[1],
        source.owner_eye_position.z as f32 + hand_offset[2],
    ];
    fishing_hook_line_origin_offset(source, hand)
}

fn fishing_hook_line_origin_offset(
    source: FishingHookRenderSourceState,
    hand: [f32; 3],
) -> [f32; 3] {
    [
        hand[0] - source.hook_position.x as f32,
        hand[1] - (source.hook_position.y as f32 + 0.25),
        hand[2] - source.hook_position.z as f32,
    ]
}

fn rotate_y(value: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    [
        value[0] * cos + value[2] * sin,
        value[1],
        value[2] * cos - value[0] * sin,
    ]
}

fn rotate_x(value: [f32; 3], angle: f32) -> [f32; 3] {
    let (sin, cos) = angle.sin_cos();
    [
        value[0],
        value[1] * cos - value[2] * sin,
        value[1] * sin + value[2] * cos,
    ]
}

/// Vanilla `LivingEntityRenderer.setupRotations` body shake, folded into the
/// projected body yaw: `cos(Mth.floor(ageInTicks) * 3.25) * π * 0.4` degrees while
/// the entity `isShaking`, otherwise `0`. `age_ticks` is the integer tick count,
/// so it already equals `Mth.floor(ageInTicks)`.
pub(super) fn entity_body_shake_degrees(age_ticks: u32, is_shaking: bool) -> f32 {
    if !is_shaking {
        return 0.0;
    }
    (age_ticks as f32 * 3.25).cos() * std::f32::consts::PI * 0.4
}

/// Vanilla `LivingEntityRenderer.isShaking`: the base renderer's `isFullyFrozen`,
/// plus the per-renderer conversion overrides. `AbstractZombieRenderer.isShaking`
/// ORs in `Zombie.isUnderWaterConverting()` (synced `DATA_DROWNED_CONVERSION_ID`,
/// id 18) for its registered Zombie, Husk, and Drowned renderers;
/// `ZombieVillagerRenderer` independently ORs in `ZombieVillager.isConverting()`
/// (synced `DATA_CONVERTING_ID`, id 19).
/// `StriderRenderer.isShaking` additionally ORs in `Strider.isSuffocating()`
/// (synced `DATA_SUFFOCATING`, id 19), the same flag that selects the cold
/// texture. `PiglinRenderer` / `HoglinRenderer` OR in `isConverting()`, which is
/// true when the entity is not immune to zombification and the dimension's
/// position's `EnvironmentAttributes.PIGLINS_ZOMBIFY` value is true. `NoAI` is not synced to
/// this client projection, matching the rest of the network-visible entity state.
/// `AbstractSkeletonRenderer` also projects the base skeleton's synced freeze-conversion boolean;
/// the server-only countdown is not needed for the client shake.
pub(super) fn entity_shaking(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    is_fully_frozen: bool,
    piglins_zombify: impl FnOnce() -> bool,
) -> bool {
    if is_fully_frozen {
        return true;
    }
    if bbb_client_world::vanilla_entity_renderer_extracts_zombie_underwater_converting(
        entity_type_id,
    ) && vanilla_entity_metadata_zombie_accessors_for_entity_type(entity_type_id).is_some_and(
        |accessors| {
            entity_data_bool(
                data_values,
                accessors.drowned_conversion_data_id,
                accessors.drowned_conversion_default,
            )
        },
    ) {
        return true;
    }
    if let Some((data_id, default)) =
        bbb_client_world::vanilla_entity_metadata_skeleton_freeze_conversion_accessor_for_entity_type(
            entity_type_id,
        )
    {
        if entity_data_bool(data_values, data_id, default) {
            return true;
        }
    }
    if let Some((data_id, default)) =
        bbb_client_world::vanilla_entity_metadata_strider_suffocating_accessor_for_entity_type(
            entity_type_id,
        )
    {
        if entity_data_bool(data_values, data_id, default) {
            return true;
        }
    }
    if let Some((data_id, default)) =
        bbb_client_world::vanilla_entity_metadata_zombie_villager_converting_accessor_for_entity_type(
            entity_type_id,
        )
    {
        if entity_data_bool(data_values, data_id, default) {
            return true;
        }
    }
    let Some((data_id, default)) =
        bbb_client_world::vanilla_entity_metadata_zombification_immunity_accessor_for_entity_type(
            entity_type_id,
        )
    else {
        return false;
    };
    !entity_data_bool(data_values, data_id, default) && piglins_zombify()
}

pub(super) fn world_piglins_zombify(world: &WorldStore, position: [f64; 3]) -> bool {
    if let Some(value) = world
        .sampled_client_environment_attributes_at(
            position,
            bbb_client_world::EnvironmentBiomeSampling::Direct,
        )
        .ok()
        .and_then(|attributes| attributes.piglins_zombify)
    {
        return value;
    }
    if world
        .registry_content(bbb_client_world::DIMENSION_TYPE_REGISTRY)
        .is_some()
    {
        return true;
    }
    let Some(level) = world.level_info() else {
        return true;
    };
    let dimension = level.dimension.as_str();
    let dimension_type = level.dimension_type_name.as_deref();
    !matches!(
        (level.dimension_type_id, dimension, dimension_type),
        (1, _, _) | (_, "minecraft:the_nether", _) | (_, _, Some("minecraft:the_nether"))
    )
}

/// Vanilla `Mth.wrapDegrees`: wraps an angle in degrees to `-180.0..=180.0`.
pub(super) fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

pub(super) fn is_squid_entity_type(entity_type_id: i32) -> bool {
    bbb_client_world::vanilla_entity_type_is_squid(entity_type_id)
}

/// Vanilla `CreeperRenderer.getWhiteOverlayProgress`: with `step` =
/// `Creeper.getSwelling`, returns `0.0` while `(int)(step * 10) % 2 == 0` and
/// `clamp(step, 0.5, 1.0)` otherwise, so the creeper strobes white as the fuse
/// nears detonation.
pub(super) fn creeper_white_overlay_progress(swelling: f32) -> f32 {
    if (swelling * 10.0) as i32 % 2 == 0 {
        0.0
    } else {
        swelling.clamp(0.5, 1.0)
    }
}
