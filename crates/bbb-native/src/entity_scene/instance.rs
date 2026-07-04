use super::*;

pub(super) fn entity_model_instance(
    source: EntityModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    game_time: i64,
    entity_partial_tick: f32,
    chicken_variants: Option<&RegistryContentState>,
    cow_variants: Option<&RegistryContentState>,
    pig_variants: Option<&RegistryContentState>,
    frog_variants: Option<&RegistryContentState>,
    cat_variants: Option<&RegistryContentState>,
    wolf_variants: Option<&RegistryContentState>,
    villager_types: Option<&RegistryContentState>,
    villager_professions: Option<&RegistryContentState>,
) -> Option<EntityModelInstance> {
    let mut kind = entity_model_kind_with_time_and_registries(
        source.entity_type_id,
        &source.data_values,
        source.age_ticks as f32 + entity_partial_tick,
        game_time,
        chicken_variants,
        cow_variants,
        pig_variants,
        frog_variants,
        cat_variants,
        wolf_variants,
    );
    // Vanilla `Armadillo.shouldHideInShell()` = `getState().shouldHideInShell(inStateTicks)`: the
    // shell-ball swap is gated on the client `inStateTicks`, which `entity_model_kind` (data-only)
    // cannot see, so it falls back to the steady SCARED hide. Override it with the world-projected
    // `isHidingInShell`, which also covers the ROLLING/UNROLLING transition windows.
    if let EntityModelKind::Armadillo { rolled_up, .. } = &mut kind {
        *rolled_up = source.armadillo_is_hiding_in_shell;
    }
    apply_player_profile_skin(&mut kind, &source, world, item_runtime);
    let player_cape_texture = player_profile_texture(
        &source,
        world,
        item_runtime,
        EntityDynamicPlayerTextureKind::Cape,
    );
    let player_elytra_texture = player_profile_texture(
        &source,
        world,
        item_runtime,
        EntityDynamicPlayerTextureKind::Elytra,
    );
    let (chest_wings_layer, chest_equipment_has_wings, chest_equipment_has_humanoid) =
        chest_equipment_layers(&source, world, item_runtime);
    // Only skeletons drive the `BOW_AND_ARROW` aim pose; resolve the held item just for them to avoid a
    // per-entity item lookup for every mob.
    let main_hand_holds_bow =
        matches!(
            kind,
            EntityModelKind::Skeleton | EntityModelKind::SkeletonVariant { .. }
        ) && entity_main_hand_holds_bow(world, item_runtime, source.entity_id);
    let is_player = matches!(kind, EntityModelKind::Player { .. });
    let main_hand_holds_spear =
        entity_hand_holds_spear(world, item_runtime, source.entity_id, false);
    let off_hand_holds_spear = entity_hand_holds_spear(world, item_runtime, source.entity_id, true);
    let main_hand_swing_is_stab_type =
        entity_hand_swing_is_stab(world, item_runtime, source.entity_id, false);
    let off_hand_swing_is_stab_type =
        entity_hand_swing_is_stab(world, item_runtime, source.entity_id, true);
    let main_hand_swing_is_none_type = entity_hand_swing_is_none(world, source.entity_id, false);
    let off_hand_swing_is_none_type = entity_hand_swing_is_none(world, source.entity_id, true);
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
    let player_main_hand_holds_charged_crossbow = is_player
        && entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, false);
    let player_off_hand_holds_charged_crossbow = is_player
        && entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, true);
    // Vanilla `AvatarRenderer.getArmPose` resolves a non-swinging charged main-hand crossbow before the
    // use-item branch, then `getArmPose(avatar, arm)` forces any non-empty off-hand pose to ITEM because
    // `CROSSBOW_HOLD.isTwoHanded()`. `HumanoidModel.setupAnim` applies that off-hand ITEM first and the
    // main-hand `CROSSBOW_HOLD` last, so the crossbow hold is the visible pose even if the off hand holds or
    // uses another special-pose item.
    let player_main_hand_crossbow_hold_pose =
        is_player && !source.is_swinging && player_main_hand_holds_charged_crossbow;
    let main_hand_crossbow_hold_forces_offhand_item =
        player_main_hand_crossbow_hold_pose && entity_offhand_non_empty(world, source.entity_id);
    let using_offhand_forced_to_item_by_main_hand_crossbow =
        source.use_item_off_hand && main_hand_crossbow_hold_forces_offhand_item;
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` selects the `attackArm` hand stack and
    // copies its `getSwingAnimation().type()` into the render state: a spear swing uses `STAB`, a patched
    // `NONE` stack keeps the shared body/anchor swing prologue but skips the WHACK/STAB arm attack.
    let main_hand_swing_is_stab = if source.attack_arm_off_hand {
        off_hand_swing_is_stab_type
    } else {
        main_hand_swing_is_stab_type
    };
    let main_hand_swing_is_none = if source.attack_arm_off_hand {
        off_hand_swing_is_none_type
    } else {
        main_hand_swing_is_none_type
    };
    // Vanilla `AvatarRenderer.getArmPose` use-item `ItemUseAnimation.SPEAR`: while the using hand holds a
    // spear, `HumanoidModel.ArmPose.SPEAR` applies `SpearAnimations.thirdPersonHandUse`, and
    // `ItemInHandLayer` applies `thirdPersonUseItem` using the spear's default `KineticWeapon`.
    let player_using_spear =
        if is_player && source.is_using_item && !using_offhand_forced_to_item_by_main_hand_crossbow
        {
            entity_hand_spear_kinetic_weapon(
                world,
                item_runtime,
                source.entity_id,
                source.use_item_off_hand,
            )
        } else {
            None
        };
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `SPYGLASS`: while a player is using a spyglass
    // (`isUsingItem` + the using hand holds a spyglass), the holding arm raises it to the eye. Only
    // `PlayerModel` consumes the use-item poses, so resolve the using-hand item just for the player kind.
    let player_using_spyglass = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_holds_spyglass(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `TOOT_HORN`: while a player is tooting a goat
    // horn (`isUsingItem` + the using hand holds a goat horn), the holding arm raises it to the mouth.
    let player_tooting_horn = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_holds_goat_horn(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `BRUSH`: while a player is brushing
    // (`isUsingItem` + the using hand holds a brush), the holding arm lowers to the brushed block.
    let player_brushing = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_holds_brush(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `BLOCK` (`poseBlockingArm`): while a player raises a
    // non-consumable `BLOCKS_ATTACKS` item (the vanilla shield or a datapack/patch-granted blocker), the
    // holding arm tucks the blocking item forward along the head look.
    let player_blocking = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_blocks_attacks(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.setupAnim` use-item arm pose `THROW_TRIDENT`: while a player charges a trident
    // throw (`isUsingItem` + the using hand holds a trident, whose `TridentItem.getUseAnimation()` is
    // `TRIDENT`), the holding arm raises the trident straight overhead. Same `poseRightArm`/`poseLeftArm`
    // case the drowned reaches via aggression, here driven by the player use-item path.
    let player_throwing_trident = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_holds_trident(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `BOW_AND_ARROW` use-item arm pose: while a player
    // draws a bow (`isUsingItem` + the using hand holds a bow, `BowItem.getUseAnimation() == BOW`), BOTH arms
    // raise along the head look. The pose is two-handed and `affectsOffhandPose`, so `poseRightArm` sets both
    // arms and the opposite arm's pose is skipped. The renderer mirrors the brace yaw when the using hand is
    // off hand.
    let player_drawing_bow = matches!(kind, EntityModelKind::Player { .. })
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_holds_bow(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        );
    // Vanilla `HumanoidModel.poseRightArm` / `poseLeftArm` `CROSSBOW_CHARGE` use-item arm pose
    // (`AnimationUtils.animateCrossbowCharge`, the same one the pillager/piglin use): while a player draws an
    // UNCHARGED crossbow (`isUsingItem` + the using hand holds a crossbow,
    // `CrossbowItem.getUseAnimation() == CROSSBOW`), the holding arm braces and the opposite arm pulls the
    // string back over the draw ticks
    // (`crossbow_charge_ticks`, the shared `getTicksUsingItem` counter advanced off `isUsingItem` in the
    // world tick loop). A CHARGED crossbow is excluded (`getArmPose` returns `CROSSBOW_HOLD` first).
    let player_charging_crossbow = is_player
        && source.is_using_item
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && entity_hand_holds_crossbow(
            world,
            item_runtime,
            source.entity_id,
            source.use_item_off_hand,
        )
        && !(if source.use_item_off_hand {
            player_off_hand_holds_charged_crossbow
        } else {
            player_main_hand_holds_charged_crossbow
        });
    // Vanilla `HumanoidModel.setupAnim` dispatch (lines 245-257): when a hand uses an `affectsOffhandPose`
    // item (the two-handed draws — `BOW_AND_ARROW` / `THROW_TRIDENT` / `CROSSBOW_CHARGE` / `CROSSBOW_HOLD`,
    // plus `SPEAR`), the OPPOSITE arm's `poseArm` is SKIPPED, so the opposite hand gets NO pose. Compute
    // this before the held-spear / crossbow-hold / item fallback flags so those projections share the same
    // vanilla skip. `SPYGLASS`/`TOOT_HORN`/`BRUSH`/`BLOCK` are NOT `affectsOffhandPose`.
    let main_hand_use_affects_offhand = source.is_using_item
        && !source.use_item_off_hand
        && (entity_hand_holds_bow(world, item_runtime, source.entity_id, false)
            || entity_hand_holds_trident(world, item_runtime, source.entity_id, false)
            || entity_hand_holds_crossbow(world, item_runtime, source.entity_id, false)
            || player_main_hand_holds_spear);
    let off_hand_use_affects_offhand = source.is_using_item
        && source.use_item_off_hand
        && !using_offhand_forced_to_item_by_main_hand_crossbow
        && (entity_hand_holds_bow(world, item_runtime, source.entity_id, true)
            || entity_hand_holds_trident(world, item_runtime, source.entity_id, true)
            || entity_hand_holds_crossbow(world, item_runtime, source.entity_id, true)
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
        && !(source.is_using_item && !source.use_item_off_hand)
        && !off_hand_use_affects_offhand
        && (source.is_using_item
            || !(player_off_hand_holds_spear || player_off_hand_stab_swing_pose));
    let player_off_hand_spear_pose = is_player
        && (player_off_hand_holds_spear || player_off_hand_stab_swing_pose)
        && !(source.is_using_item && source.use_item_off_hand)
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
    let main_hand_use_is_special = source.is_using_item
        && !source.use_item_off_hand
        && entity_hand_holds_special_use_item(world, item_runtime, source.entity_id, false);
    let off_hand_use_is_special = source.is_using_item
        && source.use_item_off_hand
        && entity_hand_holds_special_use_item(world, item_runtime, source.entity_id, true);
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
        && (source.is_using_item || !player_off_hand_holds_spear)
        && entity_main_hand_non_empty(world, source.entity_id)
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
            && entity_offhand_non_empty(world, source.entity_id)
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
    ) && entity_holds_crossbow(world, item_runtime, source.entity_id);
    // Vanilla `IllagerModel.setupAnim` `ATTACKING` branch selects empty-handed `animateZombieArms` versus
    // armed `swingWeaponDown` from the rendered main-hand item state.
    let illager_main_hand_empty = matches!(kind, EntityModelKind::Illager { .. })
        && !entity_main_hand_non_empty(world, source.entity_id);
    // Vanilla `DrownedRenderer.getArmPose`: a drowned in its main hand holding a trident while aggressive
    // (`getMainArm() == arm && isAggressive() && item.is(Items.TRIDENT)`) raises the trident overhead to
    // throw it. `isAggressive` is already projected (the drowned is in the zombie model family); resolve
    // the held item just for the drowned.
    let drowned_throw_trident = matches!(
        kind,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            ..
        }
    ) && source.is_aggressive
        && entity_hand_holds_trident(world, item_runtime, source.entity_id, false);
    // Vanilla `WitchRenderer.extractRenderState`: `isHoldingItem` is any non-empty main hand and
    // `isHoldingPotion` is exactly `Items.POTION`. The former drives the witch model's nose hold pose; the
    // latter selects `WitchItemLayer`'s nose-attached potion branch.
    let witch_holding_item = matches!(kind, EntityModelKind::Witch)
        && entity_main_hand_non_empty(world, source.entity_id);
    let witch_holding_potion = witch_holding_item
        && entity_main_hand_is_item(world, item_runtime, source.entity_id, "minecraft:potion");
    // Vanilla `CopperGolemModel.setupAnim`: either rendered hand item state selects the held-item arm
    // clamp before `ItemInHandLayer` reads the hand transform. The walk-with-item keyframe stays deferred.
    let copper_golem_holding_item = matches!(kind, EntityModelKind::CopperGolem { .. })
        && (entity_main_hand_non_empty(world, source.entity_id)
            || entity_offhand_non_empty(world, source.entity_id));
    let custom_head_skull = entity_custom_head_skull(world, item_runtime, source.entity_id);
    // Vanilla `Piglin.getArmPose` `ADMIRING_ITEM` (`PiglinAi.isLovedItem(getOffhandItem())`): a regular
    // piglin holding a piglin-loved item in its OFFHAND admires it (head tilts down, the off arm lifts the
    // item). Second-highest priority (below DANCING, above ATTACKING / CROSSBOW), so it suppresses those.
    // Only `Piglin.getArmPose` has this branch — the brute's is ATTACKING/DEFAULT only — so gate to the
    // regular piglin. Resolve the offhand item + the `minecraft:piglin_loved` item tag just for it.
    let piglin_admiring = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin,
            ..
        }
    ) && !piglin_is_dancing(source.entity_type_id, &source.data_values)
        && entity_offhand_holds_loved_item(world, source.entity_id);
    // Vanilla `Piglin.getArmPose` `CROSSBOW_HOLD`: a regular piglin holding a charged crossbow, not
    // dancing (top priority), not admiring (an offhand loved item, higher priority), and not mid-draw
    // (`CROSSBOW_CHARGE`, whose pull-back pose is deferred). The higher-priority `ATTACKING_WITH_MELEE_WEAPON`
    // needs a tool main-hand item, so a charged-crossbow hand excludes it. Resolve the held item just for it.
    let piglin_crossbow_hold =
        matches!(
            kind,
            EntityModelKind::Piglin {
                family: PiglinModelFamily::Piglin,
                ..
            }
        ) && entity_hand_holds_charged_crossbow(world, item_runtime, source.entity_id, false)
            && !piglin_is_charging_crossbow(source.entity_type_id, &source.data_values)
            && !piglin_is_dancing(source.entity_type_id, &source.data_values)
            && !piglin_admiring;
    // Vanilla `Piglin`/`PiglinBrute.getArmPose` `ATTACKING_WITH_MELEE_WEAPON`: a piglin or piglin brute
    // that is aggressive (`Mob.isAggressive()`) and holds a melee weapon (`isHoldingMeleeWeapon()`, a
    // main-hand item with the `tool` component), not dancing, and (for the regular piglin) not admiring an
    // offhand loved item (both higher priority). The brute has no dance/admire/crossbow poses, so
    // `piglin_admiring` is always false for it. The zombified piglin uses its renderer zombie-arm pose
    // instead of this weapon-raised pose, so it is excluded. Resolve the held item just for these families.
    let piglin_attacking_with_melee = matches!(
        kind,
        EntityModelKind::Piglin {
            family: PiglinModelFamily::Piglin | PiglinModelFamily::PiglinBrute,
            ..
        }
    ) && source.is_aggressive
        && entity_main_hand_holds_melee_weapon(world, source.entity_id)
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
    // Vanilla `Goat.getRammingXHeadRot()`: the world-projected `lowerHeadTick` ram counter scaled by the
    // adult/baby max head pitch. Resolved here because the baby flag lives in the goat kind.
    let goat_ramming_x_head_rot = match kind {
        EntityModelKind::Goat { baby, .. } => {
            goat_ramming_x_head_rot(source.goat_lower_head_tick, baby)
        }
        _ => 0.0,
    };
    let head_eat = sheep_head_eat_pose(
        source.entity_type_id,
        source.sheep_eat_animation_tick,
        entity_partial_tick,
    );
    let light_coords =
        entity_light_coords(source.entity_type_id, &source.data_values, source.light);
    // Vanilla `Camel.setupAnimationStates()`: the sit/sit-pose/stand-up timing projected purely
    // from the synced `LAST_POSE_CHANGE_TICK` (id 20) and the world game time.
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
    let head_sign = if source.is_upside_down { -1.0 } else { 1.0 };
    // Vanilla `Squid.aiStep` refines `yBodyRot` from movement independently of
    // the synced transform yaw, and `LivingEntityRenderer.extractRenderState`
    // uses that value as `bodyRot`. Other entities still use the canonical
    // synced yaw projected by WorldStore.
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
        world_piglins_zombify(world),
    );
    let body_rot = projected_body_yaw + entity_body_shake_degrees(source.age_ticks, is_shaking);
    // Vanilla LivingEntityRenderer.setupRotations riptide branch reads the lerped
    // `state.ageInTicks` (= tickCount + partialTick) only while `isAutoSpinAttack`.
    let auto_spin_age_ticks = source
        .is_auto_spin_attack
        .then_some(source.age_ticks as f32 + entity_partial_tick);
    // Vanilla `ArmedEntityRenderState.extractArmedEntityRenderState` fills Vex right/left item states
    // from `getItemHeldByArm(RIGHT/LEFT)`. bbb does not yet project non-player main-arm handedness here;
    // Vexes use the default RIGHT main arm path, so canonical main/off hand feed RIGHT/LEFT respectively.
    let is_vex = matches!(kind, EntityModelKind::Vex { .. });
    let vex_right_hand_item_non_empty =
        is_vex && entity_main_hand_non_empty(world, source.entity_id);
    let vex_left_hand_item_non_empty = is_vex && entity_offhand_non_empty(world, source.entity_id);
    // Vanilla setupRotations lifts the upside-down model by its bounding box height.
    let upside_down_height = source.is_upside_down.then_some(source.bounding_box_height);
    let drowned_bounding_box_height = matches!(
        &kind,
        EntityModelKind::ZombieVariant {
            family: ZombieVariantModelFamily::Drowned,
            ..
        }
    )
    .then_some(source.bounding_box_height)
    .unwrap_or_default();
    // Vanilla setupRotations sleeping branch: the bed yaw (or the body yaw when not
    // in a bed) plus the bed head-offset translate.
    let sleeping = source.is_sleeping.then_some(SleepingPose {
        yaw_angle: source.sleeping_bed_yaw.unwrap_or(body_rot),
        bed_offset: source.sleeping_bed_offset,
    });
    let outline_color = if source.outline_color != 0 {
        source.outline_color
    } else if source.appears_glowing {
        ENTITY_DEFAULT_OUTLINE_COLOR
    } else {
        0
    };
    Some(entity_render_state_passthrough!(
        EntityModelInstance::new(
            source.entity_id,
            kind,
            [
                source.position.x as f32,
                source.position.y as f32,
                source.position.z as f32,
            ],
            body_rot,
        )
        .with_head_eat(head_eat)
        .with_head_look(net_head_yaw, head_pitch)
        .with_trident_foil(thrown_trident_foil(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_invisible(entity_invisible(&source.data_values))
        .with_outline_color(outline_color)
        .with_light_coords(light_coords)
        .with_auto_spin_age_ticks(auto_spin_age_ticks)
        .with_upside_down_height(upside_down_height)
        .with_bounding_box_height(drowned_bounding_box_height)
        .with_sleeping(sleeping)
        .with_walk_animation(source.walk_animation_position, source.walk_animation_speed)
        .with_worn_head_animation_pos(source.worn_head_animation_position)
        .with_is_riding(source.is_passenger)
        .with_age_in_ticks(source.age_ticks as f32 + entity_partial_tick)
        .with_minecart_hurt_time(source.boat_hurt_time)
        .with_minecart_hurt_dir(source.boat_hurt_dir)
        .with_minecart_damage_time(source.boat_damage_time)
        .with_main_hand_holds_bow(main_hand_holds_bow)
        .with_main_hand_swing_is_stab(main_hand_swing_is_stab)
        .with_main_hand_swing_is_none(main_hand_swing_is_none)
        .with_player_using_spear(player_using_spear)
        .with_player_main_hand_spear_pose(player_main_hand_spear_pose)
        .with_player_off_hand_spear_pose(player_off_hand_spear_pose)
        .with_humanoid_mob_main_hand_spear_pose(humanoid_mob_main_hand_spear_pose)
        .with_humanoid_mob_off_hand_spear_pose(humanoid_mob_off_hand_spear_pose)
        .with_player_using_spyglass(player_using_spyglass)
        .with_player_tooting_horn(player_tooting_horn)
        .with_player_brushing(player_brushing)
        .with_player_blocking(player_blocking)
        .with_player_throwing_trident(player_throwing_trident)
        .with_player_drawing_bow(player_drawing_bow)
        .with_player_charging_crossbow(player_charging_crossbow)
        .with_player_crossbow_hold(player_crossbow_hold)
        .with_player_crossbow_hold_off_hand(player_crossbow_hold_off_hand)
        .with_player_main_hand_item_pose(player_main_hand_item_pose)
        .with_player_off_hand_item_pose(player_off_hand_item_pose)
        .with_player_cape_texture(player_cape_texture)
        .with_player_elytra_texture(player_elytra_texture)
        .with_chest_wings_layer(chest_wings_layer)
        .with_chest_equipment_has_wings(chest_equipment_has_wings)
        .with_chest_equipment_has_humanoid(chest_equipment_has_humanoid)
        .with_player_left_shoulder_parrot(
            source
                .player_left_shoulder_parrot
                .map(ParrotModelVariant::from_id),
        )
        .with_player_right_shoulder_parrot(
            source
                .player_right_shoulder_parrot
                .map(ParrotModelVariant::from_id),
        )
        .with_pillager_holds_crossbow(pillager_holds_crossbow)
        .with_illager_main_hand_empty(illager_main_hand_empty)
        .with_drowned_throw_trident(drowned_throw_trident)
        .with_is_charging_crossbow(pillager_is_charging_crossbow(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_witch_holding_item(witch_holding_item)
        .with_witch_holding_potion(witch_holding_potion)
        .with_copper_golem_holding_item(copper_golem_holding_item)
        .with_custom_head_skull(custom_head_skull)
        .with_bee_angry(bee_is_angry(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_camel_sit_seconds(camel_sit.sit_seconds)
        .with_camel_sit_pose_seconds(camel_sit.sit_pose_seconds)
        .with_camel_standup_seconds(camel_sit.standup_seconds)
        .with_vex_right_hand_item_non_empty(vex_right_hand_item_non_empty)
        .with_vex_left_hand_item_non_empty(vex_left_hand_item_non_empty)
        .with_wither_powered(wither_powered(source.entity_type_id, &source.data_values))
        .with_head_armor(armor_material(source.head_armor))
        .with_chest_armor(armor_material(source.chest_armor))
        .with_legs_armor(armor_material(source.legs_armor))
        .with_feet_armor(armor_material(source.feet_armor))
        .with_head_armor_dye(armor_dye(source.head_armor_dye))
        .with_chest_armor_dye(armor_dye(source.chest_armor_dye))
        .with_legs_armor_dye(armor_dye(source.legs_armor_dye))
        .with_feet_armor_dye(armor_dye(source.feet_armor_dye))
        .with_equine_body_armor(armor_material(source.equine_body_armor))
        .with_equine_body_armor_dye(armor_dye(source.equine_body_armor_dye))
        .with_wolf_body_armor(armor_material(source.wolf_body_armor))
        .with_wolf_body_armor_dye(armor_dye(source.wolf_body_armor_dye))
        .with_wolf_body_armor_crackiness(wolf_armor_crackiness(source.wolf_body_armor_crackiness))
        .with_nautilus_body_armor(armor_material(source.nautilus_body_armor))
        .with_llama_body_decor(llama_body_decor_color(source.llama_body_decor))
        .with_guardian_beam(guardian_beam(source.guardian_beam))
        .with_end_crystal_beam(end_crystal_beam(source.end_crystal_beam))
        .with_ender_dragon_beam(ender_dragon_beam(source.ender_dragon_beam))
        .with_wolf_tail_angle(wolf_tail_angle(
            source.entity_type_id,
            &source.data_values,
            game_time,
        ))
        .with_wolf_sitting(wolf_sitting(source.entity_type_id, &source.data_values))
        .with_parrot_sitting(parrot_sitting(source.entity_type_id, &source.data_values))
        .with_illager_spellcasting(illager_spellcasting(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_illager_celebrating(illager_celebrating(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_piglin_dancing(piglin_is_dancing(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_piglin_crossbow_hold(piglin_crossbow_hold)
        .with_piglin_crossbow_charge(piglin_crossbow_charge)
        .with_piglin_attacking_with_melee(piglin_attacking_with_melee)
        .with_piglin_admiring(piglin_admiring)
        .with_panda_unhappy(panda_is_unhappy(source.entity_type_id, &source.data_values))
        .with_panda_sneezing(panda_is_sneezing(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_panda_sneeze_time(panda_sneeze_time(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_panda_eating(panda_is_eating(source.entity_type_id, &source.data_values))
        .with_panda_scared(panda_is_scared(
            source.entity_type_id,
            &source.data_values,
            world,
        ))
        .with_panda_sitting(panda_is_sitting(source.entity_type_id, &source.data_values))
        .with_goat_ramming_x_head_rot(goat_ramming_x_head_rot)
        .with_turtle_has_egg(turtle_has_egg(source.entity_type_id, &source.data_values))
        .with_turtle_laying_egg(turtle_laying_egg(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_end_crystal_shows_bottom(end_crystal_shows_bottom(
            source.entity_type_id,
            &source.data_values,
        ))
        .with_creeper_powered(creeper_powered(source.entity_type_id, &source.data_values))
        .with_villager_model_data(villager_model_data(
            source.entity_type_id,
            &source.data_values,
            villager_types,
            villager_professions,
        ))
        .with_shulker_attach_face(entity_attachment_face(source.shulker_attach_face))
        .with_squid_body_tilt(source.squid_x_body_rot, source.squid_z_body_rot)
        .with_white_overlay_progress(creeper_white_overlay_progress(source.creeper_swelling)),
        source,
        with_arrow_shake arrow_shake,
        with_invisible_to_player invisible_to_player,
        with_polar_bear_stand_scale polar_bear_stand_scale,
        with_has_red_overlay has_red_overlay,
        with_death_time death_time,
        with_ender_dragon_death_time ender_dragon_death_time,
        with_scale scale,
        with_swim_amount swim_amount,
        with_in_water in_water,
        with_on_ground on_ground,
        with_is_moving is_moving,
        with_attack_anim attack_anim,
        with_main_arm_left main_arm_left,
        with_attack_arm_off_hand attack_arm_off_hand,
        with_boat_rowing_time_left boat_rowing_time_left,
        with_boat_rowing_time_right boat_rowing_time_right,
        with_boat_hurt_time boat_hurt_time,
        with_boat_hurt_dir boat_hurt_dir,
        with_boat_damage_time boat_damage_time,
        with_minecart_new_render minecart_new_render,
        with_minecart_pos_on_rail minecart_pos_on_rail,
        with_minecart_front_pos minecart_front_pos,
        with_minecart_back_pos minecart_back_pos,
        with_minecart_tnt_fuse_remaining_in_ticks minecart_tnt_fuse_remaining_in_ticks,
        with_boat_bubble_angle boat_bubble_angle,
        with_boat_underwater boat_underwater,
        with_is_aggressive is_aggressive,
        with_ticks_since_kinetic_hit_feedback ticks_since_kinetic_hit_feedback,
        with_show_extra_ears show_extra_ears,
        with_player_cape_flap player_cape_flap,
        with_player_cape_lean player_cape_lean,
        with_player_cape_lean2 player_cape_lean2,
        with_is_using_item is_using_item,
        with_use_item_off_hand use_item_off_hand,
        with_crossbow_charge_ticks crossbow_charge_ticks,
        with_enderman_carrying enderman_carrying,
        with_enderman_creepy enderman_creepy,
        with_bat_resting bat_resting,
        with_bee_has_stinger bee_has_stinger,
        with_bee_roll_amount bee_roll_amount,
        with_frog_croak_seconds frog_croak_seconds,
        with_frog_tongue_seconds frog_tongue_seconds,
        with_frog_jump_seconds frog_jump_seconds,
        with_frog_swim_idle_seconds frog_swim_idle_seconds,
        with_sniffer_animation_id sniffer_animation_id,
        with_sniffer_animation_seconds sniffer_animation_seconds,
        with_sniffer_is_searching sniffer_is_searching,
        with_armadillo_is_hiding_in_shell armadillo_is_hiding_in_shell,
        with_armadillo_roll_up_seconds armadillo_roll_up_seconds,
        with_armadillo_roll_out_seconds armadillo_roll_out_seconds,
        with_armadillo_peek_seconds armadillo_peek_seconds,
        with_fox_head_roll_angle fox_head_roll_angle,
        with_fox_crouch_amount fox_crouch_amount,
        with_fox_is_crouching fox_is_crouching,
        with_fox_is_sleeping fox_is_sleeping,
        with_fox_is_sitting fox_is_sitting,
        with_fox_is_pouncing fox_is_pouncing,
        with_fox_is_faceplanted fox_is_faceplanted,
        with_feline_is_crouching feline_is_crouching,
        with_feline_is_sprinting feline_is_sprinting,
        with_feline_is_sitting feline_is_sitting,
        with_feline_lie_down_amount feline_lie_down_amount,
        with_feline_lie_down_amount_tail feline_lie_down_amount_tail,
        with_feline_relax_state_one_amount feline_relax_state_one_amount,
        with_feline_is_lying_on_top_of_sleeping_player feline_is_lying_on_top_of_sleeping_player,
        with_camel_dash_seconds camel_dash_seconds,
        with_camel_idle_seconds camel_idle_seconds,
        with_copper_golem_idle_seconds copper_golem_idle_seconds,
        with_copper_golem_get_item_seconds copper_golem_get_item_seconds,
        with_copper_golem_get_no_item_seconds copper_golem_get_no_item_seconds,
        with_copper_golem_drop_item_seconds copper_golem_drop_item_seconds,
        with_copper_golem_drop_no_item_seconds copper_golem_drop_no_item_seconds,
        with_camel_jump_cooldown camel_jump_cooldown,
        with_vex_charging vex_charging,
        with_wither_invulnerable_ticks wither_invulnerable_ticks,
        with_wither_x_head_rots wither_x_head_rots,
        with_wither_y_head_rots wither_y_head_rots,
        with_head_armor_foil head_armor_foil,
        with_chest_armor_foil chest_armor_foil,
        with_legs_armor_foil legs_armor_foil,
        with_feet_armor_foil feet_armor_foil,
        with_pig_saddle pig_saddle,
        with_equine_saddle equine_saddle,
        with_equine_saddle_ridden equine_saddle_ridden,
        with_equine_animate_tail equine_animate_tail,
        with_equine_eat_animation equine_eat_animation,
        with_equine_stand_animation equine_stand_animation,
        with_equine_feeding_animation equine_feeding_animation,
        with_wolf_body_armor_foil wolf_body_armor_foil,
        with_strider_ridden strider_ridden,
        with_strider_saddle strider_saddle,
        with_camel_saddle camel_saddle,
        with_camel_saddle_ridden camel_saddle_ridden,
        with_nautilus_saddle nautilus_saddle,
        with_is_crouching is_crouching,
        with_elytra_rot_x elytra_rot_x,
        with_elytra_rot_y elytra_rot_y,
        with_elytra_rot_z elytra_rot_z,
        with_wolf_wet_shade wolf_wet_shade,
        with_wolf_shake_anim wolf_shake_anim,
        with_wolf_head_roll_angle wolf_head_roll_angle,
        with_parrot_party parrot_party,
        with_panda_sit_amount panda_sit_amount,
        with_panda_lie_on_back_amount panda_lie_on_back_amount,
        with_panda_roll_amount panda_roll_amount,
        with_panda_roll_time panda_roll_time,
        with_iron_golem_attack_ticks_remaining iron_golem_attack_ticks_remaining,
        with_iron_golem_offer_flower_tick iron_golem_offer_flower_tick,
        with_snow_golem_pumpkin snow_golem_pumpkin,
        with_ravager_stunned_ticks_remaining ravager_stunned_ticks_remaining,
        with_ravager_attack_ticks_remaining ravager_attack_ticks_remaining,
        with_ravager_roar_animation ravager_roar_animation,
        with_hoglin_attack_animation_tick hoglin_attack_animation_tick,
        with_armor_stand_wiggle armor_stand_wiggle,
        with_creeper_swelling creeper_swelling,
        with_villager_unhappy villager_unhappy,
        with_shulker_peek shulker_peek,
        with_tendril_animation tendril_animation,
        with_heart_animation heart_animation,
        with_warden_roar_seconds warden_roar_seconds,
        with_warden_sniff_seconds warden_sniff_seconds,
        with_warden_attack_seconds warden_attack_seconds,
        with_warden_sonic_boom_seconds warden_sonic_boom_seconds,
        with_warden_emerge_seconds warden_emerge_seconds,
        with_warden_dig_seconds warden_dig_seconds,
        with_rabbit_hop_seconds rabbit_hop_seconds,
        with_creaking_can_move creaking_can_move,
        with_creaking_attack_seconds creaking_attack_seconds,
        with_creaking_invulnerable_seconds creaking_invulnerable_seconds,
        with_creaking_death_seconds creaking_death_seconds,
        with_squid_tentacle_angle squid_tentacle_angle,
        with_guardian_tail_animation guardian_tail_animation,
        with_guardian_spikes_animation guardian_spikes_animation,
        with_breeze_shoot_seconds breeze_shoot_seconds,
        with_breeze_slide_seconds breeze_slide_seconds,
        with_breeze_slide_back_seconds breeze_slide_back_seconds,
        with_breeze_inhale_seconds breeze_inhale_seconds,
        with_breeze_long_jump_seconds breeze_long_jump_seconds,
        with_chicken_flap chicken_flap,
        with_chicken_flap_speed chicken_flap_speed,
        with_slime_squish slime_squish,
        with_evoker_fangs_bite_progress evoker_fangs_bite_progress,
        with_allay_dancing allay_dancing,
        with_allay_spinning allay_spinning,
        with_allay_spinning_progress allay_spinning_progress,
        with_allay_holding_item_progress allay_holding_item_progress,
        with_axolotl_playing_dead_factor axolotl_playing_dead_factor,
        with_axolotl_in_water_factor axolotl_in_water_factor,
        with_axolotl_on_ground_factor axolotl_on_ground_factor,
        with_axolotl_moving_factor axolotl_moving_factor,
        with_parrot_flap_angle parrot_flap_angle,
    ))
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
/// id 18) for the whole zombie family, and `ZombieVillagerRenderer` additionally
/// ORs in `ZombieVillager.isConverting()` (synced `DATA_CONVERTING_ID`, id 19).
/// `StriderRenderer.isShaking` additionally ORs in `Strider.isSuffocating()`
/// (synced `DATA_SUFFOCATING`, id 19), the same flag that selects the cold
/// texture. `PiglinRenderer` / `HoglinRenderer` OR in `isConverting()`, which is
/// true when the entity is not immune to zombification and the dimension's
/// `EnvironmentAttributes.PIGLINS_ZOMBIFY` value is true. `NoAI` is not synced to
/// this client projection, matching the rest of the network-visible entity state.
/// The base-`Skeleton` freeze-conversion shake remains deferred because it is a
/// server-side `conversionTime`.
pub(super) fn entity_shaking(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    is_fully_frozen: bool,
    piglins_zombify: bool,
) -> bool {
    if is_fully_frozen {
        return true;
    }
    match entity_type_id {
        VANILLA_ENTITY_TYPE_ZOMBIE_ID
        | VANILLA_ENTITY_TYPE_HUSK_ID
        | VANILLA_ENTITY_TYPE_DROWNED_ID => {
            entity_data_bool(data_values, ZOMBIE_DROWNED_CONVERSION_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID => {
            entity_data_bool(data_values, ZOMBIE_DROWNED_CONVERSION_DATA_ID, false)
                || entity_data_bool(data_values, ZOMBIE_VILLAGER_CONVERTING_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_STRIDER_ID => {
            entity_data_bool(data_values, STRIDER_SUFFOCATING_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_PIGLIN_ID | VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID => {
            piglins_zombify
                && !entity_data_bool(data_values, PIGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID, false)
        }
        VANILLA_ENTITY_TYPE_HOGLIN_ID => {
            piglins_zombify
                && !entity_data_bool(data_values, HOGLIN_IMMUNE_TO_ZOMBIFICATION_DATA_ID, false)
        }
        _ => false,
    }
}

/// Vanilla `EnvironmentAttributes.PIGLINS_ZOMBIFY` defaults to `true`; the built-in Nether
/// dimension type explicitly sets it to `false`. bbb currently projects built-in dimension
/// profiles by id/name/type-name, so custom dimension attribute maps stay a later resource-registry
/// parity item.
pub(super) fn world_piglins_zombify(world: &WorldStore) -> bool {
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
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_SQUID_ID | VANILLA_ENTITY_TYPE_GLOW_SQUID_ID
    )
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

/// Packs the entity's sampled block+sky light into vanilla
/// `EntityRenderState.lightCoords` (`LightCoordsUtil.pack(block, sky)`). Mirrors
/// `EntityRenderer.getBlockLightLevel`, which forces block light to `15` while
/// the entity is on fire (shared-flags bit `0x01`); sky light is unchanged.
/// Vanilla `GlowSquid.DATA_DARK_TICKS_REMAINING` synced INT (the first own accessor on the
/// `Squid`/`AgeableWaterCreature` chain, so index `18`: Entity 0-7, LivingEntity 8-14, Mob 15,
/// AgeableMob baby 16 + age-locked 17). Counts down from `100` after a hurt; `0` while undamaged.
pub(super) const GLOW_SQUID_DARK_TICKS_DATA_ID: u8 = 18;

/// Vanilla `Mth.clampedLerp(factor, min, max)`: `min` for `factor < 0`, `max` for `factor > 1`, else the
/// linear interpolation `min + factor·(max − min)`.
pub(super) fn clamped_lerp(factor: f32, min: f32, max: f32) -> f32 {
    if factor < 0.0 {
        min
    } else if factor > 1.0 {
        max
    } else {
        min + factor * (max - min)
    }
}

pub(super) fn entity_light_coords(
    entity_type_id: i32,
    data_values: &[bbb_protocol::packets::EntityDataValue],
    light: bbb_world::TerrainLight,
) -> u32 {
    let on_fire = (entity_data_byte(data_values, ENTITY_SHARED_FLAGS_DATA_ID, 0)
        & ENTITY_SHARED_FLAG_ON_FIRE)
        != 0;
    let mut block = if on_fire {
        15
    } else {
        u32::from(light.block.min(15))
    };
    // Vanilla full-bright renderers (`getBlockLightLevel` returns `15` unconditionally): these glow with
    // their own internal fire / energy regardless of the surrounding block light. The complete 26.1 set is
    // `BlazeRenderer`, `MagmaCubeRenderer`, `WitherBossRenderer`, `WitherSkullRenderer`,
    // `DragonFireballRenderer`, `ShulkerBulletRenderer`, `AllayRenderer`, and `VexRenderer`.
    if matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_BLAZE_ID
            | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            | VANILLA_ENTITY_TYPE_WITHER_ID
            | VANILLA_ENTITY_TYPE_WITHER_SKULL_ID
            | VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID
            | VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
            | VANILLA_ENTITY_TYPE_ALLAY_ID
            | VANILLA_ENTITY_TYPE_VEX_ID
    ) {
        block = 15;
    }
    // Vanilla `ItemFrameRenderer.getBlockLightLevel`: glow item frames keep their surrounding light, but
    // raise the block component to at least `GLOW_FRAME_BRIGHTNESS = 5`.
    if entity_type_id == VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID {
        block = block.max(5);
    }
    // Vanilla `GlowSquidRenderer.getBlockLightLevel`: a bioluminescent boost
    // `max(blockLight, (int)clampedLerp(1 − darkTicks/10, 0, 15))`. Undamaged (`darkTicks == 0`) it is fully
    // bright (`15`); a hurt drops `darkTicks` to `100` (dark), and it ramps back to full over the final 10
    // ticks. The vanilla `glow == 15 ? 15 : max(glow, super)` ternary is just `max(super, glow)` (super ≤ 15).
    if entity_type_id == VANILLA_ENTITY_TYPE_GLOW_SQUID_ID {
        let dark_ticks = entity_data_int(data_values, GLOW_SQUID_DARK_TICKS_DATA_ID, 0);
        let glow = clamped_lerp(1.0 - dark_ticks as f32 / 10.0, 0.0, 15.0) as u32;
        block = block.max(glow);
    }
    let sky = u32::from(light.sky.min(15));
    block << 4 | sky << 20
}

/// Projects the canonical sheep `eatAnimationTick` into the renderer head-eat
/// pose. Vanilla `SheepRenderer.extractRenderState` calls
/// `Sheep.getHeadEatPositionScale`/`getHeadEatAngleScale` with the partial tick;
/// every non-sheep entity resolves to [`SheepHeadEatPose::NONE`].
pub(super) fn sheep_head_eat_pose(
    entity_type_id: i32,
    sheep_eat_animation_tick: i32,
    partial_tick: f32,
) -> SheepHeadEatPose {
    if entity_type_id == VANILLA_ENTITY_TYPE_SHEEP_ID {
        SheepHeadEatPose::from_eat_tick(sheep_eat_animation_tick, partial_tick)
    } else {
        SheepHeadEatPose::NONE
    }
}
