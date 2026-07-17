//! Source-derived vanilla entity renderer registration behavior.
//!
//! This owner is intentionally separate from entity metadata: registrations
//! and renderer overrides are client presentation facts, while metadata schema
//! facts follow the server-synchronized entity class hierarchy.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VanillaEntityRendererRegistration {
    entity_type_id: i32,
    constant_name: &'static str,
    renderer_class: &'static str,
    static_full_block_light: bool,
    minimum_block_light: Option<u8>,
    thrown_item_scale_bits: Option<u32>,
    uses_abstract_feline_model: bool,
    uses_abstract_horse_renderer: bool,
    uses_mob_item_in_hand_layer: bool,
    extracts_mob_aggressive: bool,
    extracts_unhappy_counter: bool,
    extracts_casting_spell: bool,
    extracts_sheep_head_eat: bool,
    extracts_zombie_underwater_converting: bool,
    extracts_wolf_sitting: bool,
    extracts_parrot_pose: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaSkeletonRendererKind {
    Skeleton,
    Stray,
    Parched,
    WitherSkeleton,
    Bogged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaIllagerRendererKind {
    Evoker,
    Illusioner,
    Pillager,
    Vindicator,
}

include!("renderer_catalog/generated_26_1.rs");

pub(super) fn vanilla_living_entity_renderer_name_is_upside_down(name: &str) -> bool {
    VANILLA_LIVING_ENTITY_RENDERER_UPSIDE_DOWN_NAMES_26_1.contains(&name)
}

fn vanilla_entity_renderer_registration(
    entity_type_id: i32,
) -> Option<&'static VanillaEntityRendererRegistration> {
    let index = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
        .binary_search_by_key(&entity_type_id, |entry| entry.entity_type_id)
        .ok()?;
    Some(&VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1[index])
}

pub fn vanilla_entity_renderer_has_static_full_block_light(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.static_full_block_light)
}

pub fn vanilla_entity_renderer_minimum_block_light(entity_type_id: i32) -> Option<u8> {
    vanilla_entity_renderer_registration(entity_type_id).and_then(|entry| entry.minimum_block_light)
}

pub fn vanilla_entity_renderer_is_noop(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.renderer_class == "NoopRenderer")
}

pub fn vanilla_entity_renderer_thrown_item_scale(entity_type_id: i32) -> Option<f32> {
    vanilla_entity_renderer_registration(entity_type_id)
        .and_then(|entry| entry.thrown_item_scale_bits)
        .map(f32::from_bits)
}

pub fn vanilla_entity_renderer_thrown_item_scales() -> impl Iterator<Item = (i32, f32)> {
    VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
        .iter()
        .filter_map(|entry| {
            entry
                .thrown_item_scale_bits
                .map(|bits| (entry.entity_type_id, f32::from_bits(bits)))
        })
}

/// Whether the registered vanilla renderer submits through a non-entity-model pass that bbb owns
/// separately (block models, item clusters/frames, firework items, or lightning geometry).
///
/// The renderer classes are stable source identities; membership is still resolved through the
/// generated `EntityRenderers` provider catalog, so a provider change cannot leave a parallel
/// entity-type table stale.
pub fn vanilla_entity_renderer_uses_native_non_model_pass(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id).is_some_and(|entry| {
        matches!(
            entry.renderer_class,
            "FallingBlockRenderer"
                | "FireworkEntityRenderer"
                | "ItemFrameRenderer"
                | "ItemEntityRenderer"
                | "LightningBoltRenderer"
                | "OminousItemSpawnerRenderer"
                | "TntRenderer"
        )
    })
}

pub(super) fn vanilla_entity_renderer_uses_abstract_feline_model(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.uses_abstract_feline_model)
}

pub(super) fn vanilla_entity_renderer_uses_abstract_horse_renderer(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.uses_abstract_horse_renderer)
}

pub(super) fn vanilla_entity_renderer_uses_mob_item_in_hand_layer(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.uses_mob_item_in_hand_layer)
}

pub(super) fn vanilla_entity_renderer_extracts_mob_aggressive(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_mob_aggressive)
}

pub(super) fn vanilla_entity_renderer_extracts_unhappy_counter(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_unhappy_counter)
}

pub(super) fn vanilla_entity_renderer_extracts_casting_spell(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_casting_spell)
}

pub fn vanilla_entity_renderer_extracts_sheep_head_eat(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_sheep_head_eat)
}

pub fn vanilla_entity_renderer_extracts_zombie_underwater_converting(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_zombie_underwater_converting)
}

pub fn vanilla_entity_renderer_extracts_wolf_sitting(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_wolf_sitting)
}

pub fn vanilla_entity_renderer_extracts_parrot_pose(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.extracts_parrot_pose)
}

pub fn vanilla_entity_renderer_skeleton_kind(
    entity_type_id: i32,
) -> Option<VanillaSkeletonRendererKind> {
    match vanilla_entity_renderer_registration(entity_type_id)?.renderer_class {
        "SkeletonRenderer" => Some(VanillaSkeletonRendererKind::Skeleton),
        "StrayRenderer" => Some(VanillaSkeletonRendererKind::Stray),
        "ParchedRenderer" => Some(VanillaSkeletonRendererKind::Parched),
        "WitherSkeletonRenderer" => Some(VanillaSkeletonRendererKind::WitherSkeleton),
        "BoggedRenderer" => Some(VanillaSkeletonRendererKind::Bogged),
        _ => None,
    }
}

pub fn vanilla_entity_renderer_illager_kind(
    entity_type_id: i32,
) -> Option<VanillaIllagerRendererKind> {
    match vanilla_entity_renderer_registration(entity_type_id)?.renderer_class {
        "EvokerRenderer" => Some(VanillaIllagerRendererKind::Evoker),
        "IllusionerRenderer" => Some(VanillaIllagerRendererKind::Illusioner),
        "PillagerRenderer" => Some(VanillaIllagerRendererKind::Pillager),
        "VindicatorRenderer" => Some(VanillaIllagerRendererKind::Vindicator),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_BLAZE_ID,
        VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_CAT_ID,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_DONKEY_ID,
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID, VANILLA_ENTITY_TYPE_DROWNED_ID,
        VANILLA_ENTITY_TYPE_EGG_ID, VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
        VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        VANILLA_ENTITY_TYPE_GIANT_ID, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_HUSK_ID,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID, VANILLA_ENTITY_TYPE_INTERACTION_ID,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
        VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID, VANILLA_ENTITY_TYPE_LINGERING_POTION_ID,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        VANILLA_ENTITY_TYPE_MARKER_ID, VANILLA_ENTITY_TYPE_MULE_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PANDA_ID,
        VANILLA_ENTITY_TYPE_PARCHED_ID, VANILLA_ENTITY_TYPE_PARROT_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
        VANILLA_ENTITY_TYPE_PILLAGER_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
        VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SKELETON_ID,
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, VANILLA_ENTITY_TYPE_SNOWBALL_ID,
        VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, VANILLA_ENTITY_TYPE_STRAY_ID,
        VANILLA_ENTITY_TYPE_TNT_ID, VANILLA_ENTITY_TYPE_TURTLE_ID, VANILLA_ENTITY_TYPE_VEX_ID,
        VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID,
        VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID, VANILLA_ENTITY_TYPE_WITCH_ID,
        VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKULL_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
    };

    use super::*;

    #[test]
    fn generated_registration_catalog_is_complete_and_unique() {
        assert_eq!(VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1.len(), 155);
        let mut ids = HashSet::new();
        let mut constants = HashSet::new();
        for entry in VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1 {
            assert!(ids.insert(entry.entity_type_id));
            assert!(constants.insert(entry.constant_name));
            assert!(!entry.renderer_class.is_empty());
        }
        assert!(!ids.contains(&VANILLA_ENTITY_TYPE_PLAYER_ID));
        assert!(!ids.contains(&VANILLA_ENTITY_TYPE_MANNEQUIN_ID));
    }

    #[test]
    fn generated_static_full_block_light_membership_matches_registered_behavior() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.static_full_block_light)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_ALLAY_ID,
                VANILLA_ENTITY_TYPE_BLAZE_ID,
                VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
                VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
                VANILLA_ENTITY_TYPE_FIREBALL_ID,
                VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
                VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
                VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
                VANILLA_ENTITY_TYPE_VEX_ID,
                VANILLA_ENTITY_TYPE_WITHER_ID,
                VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_has_static_full_block_light(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_has_static_full_block_light(-1));
        assert!(!vanilla_entity_renderer_has_static_full_block_light(157));
    }

    #[test]
    fn generated_minimum_block_light_matches_item_frame_renderer_condition() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter_map(|entry| {
                entry
                    .minimum_block_light
                    .map(|minimum| (entry.entity_type_id, minimum))
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, vec![(VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, 5)]);
        assert_eq!(
            vanilla_entity_renderer_minimum_block_light(VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID),
            Some(5)
        );
        assert_eq!(
            vanilla_entity_renderer_minimum_block_light(VANILLA_ENTITY_TYPE_ITEM_FRAME_ID),
            None
        );
        assert_eq!(vanilla_entity_renderer_minimum_block_light(-1), None);
        assert_eq!(vanilla_entity_renderer_minimum_block_light(157), None);
    }

    #[test]
    fn generated_noop_renderer_membership_matches_registration_class() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.renderer_class == "NoopRenderer")
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
                VANILLA_ENTITY_TYPE_INTERACTION_ID,
                VANILLA_ENTITY_TYPE_MARKER_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_is_noop(entity_type_id));
        }
        assert!(!vanilla_entity_renderer_is_noop(-1));
        assert!(!vanilla_entity_renderer_is_noop(157));
    }

    #[test]
    fn generated_sitting_pose_memberships_match_registered_extractors() {
        let wolf = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_wolf_sitting)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        let parrot = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_parrot_pose)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(wolf, vec![VANILLA_ENTITY_TYPE_WOLF_ID]);
        assert_eq!(parrot, vec![VANILLA_ENTITY_TYPE_PARROT_ID]);
        assert!(vanilla_entity_renderer_extracts_wolf_sitting(
            VANILLA_ENTITY_TYPE_WOLF_ID
        ));
        assert!(vanilla_entity_renderer_extracts_parrot_pose(
            VANILLA_ENTITY_TYPE_PARROT_ID
        ));
        assert!(!vanilla_entity_renderer_extracts_wolf_sitting(
            VANILLA_ENTITY_TYPE_PARROT_ID
        ));
        assert!(!vanilla_entity_renderer_extracts_parrot_pose(
            VANILLA_ENTITY_TYPE_WOLF_ID
        ));
    }

    #[test]
    fn generated_skeleton_kinds_match_registered_renderer_classes() {
        assert_eq!(
            vanilla_entity_renderer_skeleton_kind(VANILLA_ENTITY_TYPE_SKELETON_ID),
            Some(VanillaSkeletonRendererKind::Skeleton)
        );
        assert_eq!(
            vanilla_entity_renderer_skeleton_kind(VANILLA_ENTITY_TYPE_STRAY_ID),
            Some(VanillaSkeletonRendererKind::Stray)
        );
        assert_eq!(
            vanilla_entity_renderer_skeleton_kind(VANILLA_ENTITY_TYPE_PARCHED_ID),
            Some(VanillaSkeletonRendererKind::Parched)
        );
        assert_eq!(
            vanilla_entity_renderer_skeleton_kind(VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID),
            Some(VanillaSkeletonRendererKind::WitherSkeleton)
        );
        assert_eq!(
            vanilla_entity_renderer_skeleton_kind(VANILLA_ENTITY_TYPE_BOGGED_ID),
            Some(VanillaSkeletonRendererKind::Bogged)
        );
        assert_eq!(
            vanilla_entity_renderer_skeleton_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            None
        );
    }

    #[test]
    fn generated_illager_kinds_match_registered_renderer_classes() {
        assert_eq!(
            vanilla_entity_renderer_illager_kind(VANILLA_ENTITY_TYPE_EVOKER_ID),
            Some(VanillaIllagerRendererKind::Evoker)
        );
        assert_eq!(
            vanilla_entity_renderer_illager_kind(VANILLA_ENTITY_TYPE_ILLUSIONER_ID),
            Some(VanillaIllagerRendererKind::Illusioner)
        );
        assert_eq!(
            vanilla_entity_renderer_illager_kind(VANILLA_ENTITY_TYPE_PILLAGER_ID),
            Some(VanillaIllagerRendererKind::Pillager)
        );
        assert_eq!(
            vanilla_entity_renderer_illager_kind(VANILLA_ENTITY_TYPE_VINDICATOR_ID),
            Some(VanillaIllagerRendererKind::Vindicator)
        );
        assert_eq!(
            vanilla_entity_renderer_illager_kind(VANILLA_ENTITY_TYPE_WITCH_ID),
            None
        );
    }

    #[test]
    fn generated_thrown_item_renderer_scales_match_provider_constructors() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter_map(|entry| {
                entry
                    .thrown_item_scale_bits
                    .map(|bits| (entry.entity_type_id, bits))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                (VANILLA_ENTITY_TYPE_EGG_ID, 1.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_ENDER_PEARL_ID, 1.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID, 1.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, 1.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_FIREBALL_ID, 3.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_SPLASH_POTION_ID, 1.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, 1.0_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, 0.75_f32.to_bits()),
                (VANILLA_ENTITY_TYPE_SNOWBALL_ID, 1.0_f32.to_bits()),
            ]
        );
        assert_eq!(
            vanilla_entity_renderer_thrown_item_scales()
                .map(|(entity_type_id, scale)| (entity_type_id, scale.to_bits()))
                .collect::<Vec<_>>(),
            actual
        );
        for (entity_type_id, bits) in actual {
            assert_eq!(
                vanilla_entity_renderer_thrown_item_scale(entity_type_id).map(f32::to_bits),
                Some(bits)
            );
        }
        assert_eq!(vanilla_entity_renderer_thrown_item_scale(-1), None);
        assert_eq!(vanilla_entity_renderer_thrown_item_scale(157), None);
    }

    #[test]
    fn generated_native_non_model_pass_membership_matches_renderer_classes() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| {
                vanilla_entity_renderer_uses_native_non_model_pass(entry.entity_type_id)
            })
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
                VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
                VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
                VANILLA_ENTITY_TYPE_ITEM_ID,
                VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
                VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
                VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
                VANILLA_ENTITY_TYPE_TNT_ID,
            ]
        );
        assert!(!vanilla_entity_renderer_uses_native_non_model_pass(-1));
        assert!(!vanilla_entity_renderer_uses_native_non_model_pass(157));
        assert!(!vanilla_entity_renderer_uses_native_non_model_pass(
            VANILLA_ENTITY_TYPE_PLAYER_ID
        ));
    }

    #[test]
    fn generated_feline_model_membership_matches_renderer_superclass_types() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.uses_abstract_feline_model)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![VANILLA_ENTITY_TYPE_CAT_ID, VANILLA_ENTITY_TYPE_OCELOT_ID]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_uses_abstract_feline_model(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_uses_abstract_feline_model(-1));
        assert!(!vanilla_entity_renderer_uses_abstract_feline_model(157));
    }

    #[test]
    fn generated_equine_membership_matches_abstract_horse_renderer_hierarchy() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.uses_abstract_horse_renderer)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_DONKEY_ID,
                VANILLA_ENTITY_TYPE_HORSE_ID,
                VANILLA_ENTITY_TYPE_MULE_ID,
                VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_uses_abstract_horse_renderer(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_uses_abstract_horse_renderer(-1));
        assert!(!vanilla_entity_renderer_uses_abstract_horse_renderer(157));
    }

    #[test]
    fn generated_mob_item_in_hand_membership_matches_renderer_layers() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.uses_mob_item_in_hand_layer)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_ALLAY_ID,
                VANILLA_ENTITY_TYPE_BOGGED_ID,
                VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                VANILLA_ENTITY_TYPE_EVOKER_ID,
                VANILLA_ENTITY_TYPE_GIANT_ID,
                VANILLA_ENTITY_TYPE_HUSK_ID,
                VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
                VANILLA_ENTITY_TYPE_PARCHED_ID,
                VANILLA_ENTITY_TYPE_PIGLIN_ID,
                VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
                VANILLA_ENTITY_TYPE_PILLAGER_ID,
                VANILLA_ENTITY_TYPE_SKELETON_ID,
                VANILLA_ENTITY_TYPE_STRAY_ID,
                VANILLA_ENTITY_TYPE_VEX_ID,
                VANILLA_ENTITY_TYPE_VINDICATOR_ID,
                VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_uses_mob_item_in_hand_layer(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_uses_mob_item_in_hand_layer(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID
        ));
        assert!(!vanilla_entity_renderer_uses_mob_item_in_hand_layer(
            VANILLA_ENTITY_TYPE_PLAYER_ID
        ));
        assert!(!vanilla_entity_renderer_uses_mob_item_in_hand_layer(
            VANILLA_ENTITY_TYPE_TURTLE_ID
        ));
    }

    #[test]
    fn generated_aggressive_projection_matches_renderer_extractors() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_mob_aggressive)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_BOGGED_ID,
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                VANILLA_ENTITY_TYPE_EVOKER_ID,
                VANILLA_ENTITY_TYPE_HUSK_ID,
                VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
                VANILLA_ENTITY_TYPE_PARCHED_ID,
                VANILLA_ENTITY_TYPE_PILLAGER_ID,
                VANILLA_ENTITY_TYPE_SKELETON_ID,
                VANILLA_ENTITY_TYPE_STRAY_ID,
                VANILLA_ENTITY_TYPE_VINDICATOR_ID,
                VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
                VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_extracts_mob_aggressive(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_extracts_mob_aggressive(
            VANILLA_ENTITY_TYPE_GIANT_ID
        ));
        assert!(!vanilla_entity_renderer_extracts_mob_aggressive(
            VANILLA_ENTITY_TYPE_PIGLIN_ID
        ));
    }

    #[test]
    fn generated_unhappy_counter_projection_matches_renderer_extractors() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_unhappy_counter)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_PANDA_ID,
                VANILLA_ENTITY_TYPE_VILLAGER_ID,
                VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_extracts_unhappy_counter(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_extracts_unhappy_counter(
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
        ));
    }

    #[test]
    fn generated_casting_spell_projection_matches_renderer_extractors() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_casting_spell)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_EVOKER_ID,
                VANILLA_ENTITY_TYPE_ILLUSIONER_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_extracts_casting_spell(
                entity_type_id
            ));
        }
        assert!(!vanilla_entity_renderer_extracts_casting_spell(
            VANILLA_ENTITY_TYPE_VINDICATOR_ID
        ));
    }

    #[test]
    fn generated_sheep_head_eat_projection_matches_renderer_extractors() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_sheep_head_eat)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(actual, vec![VANILLA_ENTITY_TYPE_SHEEP_ID]);
        assert!(vanilla_entity_renderer_extracts_sheep_head_eat(
            VANILLA_ENTITY_TYPE_SHEEP_ID
        ));
        assert!(!vanilla_entity_renderer_extracts_sheep_head_eat(-1));
        assert!(!vanilla_entity_renderer_extracts_sheep_head_eat(157));
    }

    #[test]
    fn generated_zombie_underwater_conversion_projection_matches_renderer_extractors() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter(|entry| entry.extracts_zombie_underwater_converting)
            .map(|entry| entry.entity_type_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                VANILLA_ENTITY_TYPE_DROWNED_ID,
                VANILLA_ENTITY_TYPE_HUSK_ID,
                VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            ]
        );
        for entity_type_id in actual {
            assert!(vanilla_entity_renderer_extracts_zombie_underwater_converting(entity_type_id));
        }
        assert!(
            !vanilla_entity_renderer_extracts_zombie_underwater_converting(
                VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID
            )
        );
    }
}
