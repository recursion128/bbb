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
}

include!("renderer_catalog/generated_26_1.rs");

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

pub fn vanilla_entity_renderer_is_noop(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| entry.renderer_class == "NoopRenderer")
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
        VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_MARKER_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID, VANILLA_ENTITY_TYPE_VEX_ID,
        VANILLA_ENTITY_TYPE_WITHER_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
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
}
