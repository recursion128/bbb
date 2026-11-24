//! Source-derived vanilla entity renderer registration behavior.
//!
//! This owner is intentionally separate from entity metadata: registrations
//! and renderer overrides are client presentation facts, while metadata schema
//! facts follow the server-synchronized entity class hierarchy.

use bbb_render_types::{
    BoatModelFamily, LlamaModelFamily, PiglinModelFamily, UndeadHorseModelFamily,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VanillaEntityRendererRegistration {
    entity_type_id: i32,
    constant_name: &'static str,
    renderer_class: &'static str,
    static_full_block_light: bool,
    minimum_block_light: Option<u8>,
    thrown_item_scale_bits: Option<u32>,
    shadow_radius_bits: u32,
    shadow_strength_bits: u32,
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
    undead_horse_kind: Option<UndeadHorseModelFamily>,
    horse_kind: Option<VanillaHorseRendererKind>,
    llama_kind: Option<LlamaModelFamily>,
    boat_kind: Option<VanillaBoatRendererKind>,
    piglin_kind: Option<PiglinModelFamily>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityRendererModelKind {
    Skeleton,
    Stray,
    Parched,
    WitherSkeleton,
    Bogged,
    Evoker,
    Illusioner,
    Pillager,
    Vindicator,
    Camel,
    CamelHusk,
    UndeadHorse(UndeadHorseModelFamily),
    Horse(VanillaHorseRendererKind),
    Llama(LlamaModelFamily),
    Boat(VanillaBoatRendererKind),
    Zombie,
    Husk,
    Drowned,
    ZombieVillager,
    Piglin(PiglinModelFamily),
    Hoglin,
    Zoglin,
    Guardian,
    ElderGuardian,
    Squid,
    GlowSquid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaHorseRendererKind {
    Horse,
    Donkey,
    Mule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanillaBoatRendererKind {
    pub family: BoatModelFamily,
    pub chest: bool,
}

/// Source-derived renderer culling behavior used by visibility-first render
/// extraction. Exceptional membership follows the generated renderer-class
/// catalog rather than a second entity-type table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VanillaEntityRendererCullingKind {
    Standard,
    NoFrustum,
    Inflate([f32; 3]),
    ExtendMinYByHalfHeight,
    Display,
    ConservativeSpecial,
}

/// The single native presentation owner for a fixed vanilla entity type.
///
/// `EntityRenderers` registers every fixed type except Player and Mannequin,
/// which vanilla creates through `createAvatarRenderers`. The owner is derived
/// from that generated registration and its concrete renderer class so native
/// model selection cannot maintain a second special-pass membership table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VanillaEntityRendererPresentationOwner {
    AvatarModel,
    EntityModel,
    Noop,
    ThrownItem,
    BlockAttachment,
    FireworkItem,
    ItemFrame,
    ItemCluster,
    Lightning,
    Display,
    Painting,
    Unknown,
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

pub(super) fn vanilla_entity_renderer_block_light_floor(entity_type_id: i32) -> u8 {
    vanilla_entity_renderer_registration(entity_type_id).map_or(0, |entry| {
        if entry.static_full_block_light {
            15
        } else {
            entry.minimum_block_light.unwrap_or(0)
        }
    })
}

pub(super) fn vanilla_entity_renderer_minimum_block_light(entity_type_id: i32) -> Option<u8> {
    vanilla_entity_renderer_registration(entity_type_id).and_then(|entry| entry.minimum_block_light)
}

#[cfg(test)]
pub(crate) fn vanilla_entity_renderer_is_noop(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_renderer_presentation_owner(entity_type_id),
        VanillaEntityRendererPresentationOwner::Noop
    )
}

/// Resolves the exhaustive native presentation owner from vanilla's generated
/// renderer provider catalog and explicit avatar exception.
pub fn vanilla_entity_renderer_presentation_owner(
    entity_type_id: i32,
) -> VanillaEntityRendererPresentationOwner {
    use VanillaEntityRendererPresentationOwner as Owner;

    if matches!(
        entity_type_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID
    ) {
        return Owner::AvatarModel;
    }
    let Some(entry) = vanilla_entity_renderer_registration(entity_type_id) else {
        return Owner::Unknown;
    };
    if entry.thrown_item_scale_bits.is_some() {
        return Owner::ThrownItem;
    }
    match entry.renderer_class {
        "NoopRenderer" => Owner::Noop,
        "FallingBlockRenderer" | "TntRenderer" => Owner::BlockAttachment,
        "FireworkEntityRenderer" => Owner::FireworkItem,
        "ItemFrameRenderer" => Owner::ItemFrame,
        "ItemEntityRenderer" | "OminousItemSpawnerRenderer" => Owner::ItemCluster,
        "LightningBoltRenderer" => Owner::Lightning,
        "PaintingRenderer" => Owner::Painting,
        class if class.starts_with("DisplayRenderer.") => Owner::Display,
        _ => Owner::EntityModel,
    }
}

pub fn vanilla_entity_renderer_culling_kind(
    entity_type_id: i32,
) -> VanillaEntityRendererCullingKind {
    let Some(renderer_class) =
        vanilla_entity_renderer_registration(entity_type_id).map(|entry| entry.renderer_class)
    else {
        return VanillaEntityRendererCullingKind::Standard;
    };
    match renderer_class {
        "LightningBoltRenderer" | "EnderDragonRenderer" => {
            VanillaEntityRendererCullingKind::NoFrustum
        }
        "SnifferRenderer" => VanillaEntityRendererCullingKind::Inflate([0.6; 3]),
        "IllusionerRenderer" => VanillaEntityRendererCullingKind::Inflate([3.0, 0.0, 3.0]),
        "HappyGhastRenderer" => VanillaEntityRendererCullingKind::ExtendMinYByHalfHeight,
        "GuardianRenderer" | "ElderGuardianRenderer" | "EndCrystalRenderer" | "ShulkerRenderer" => {
            VanillaEntityRendererCullingKind::ConservativeSpecial
        }
        class if class.starts_with("DisplayRenderer.") => VanillaEntityRendererCullingKind::Display,
        _ => VanillaEntityRendererCullingKind::Standard,
    }
}

#[cfg(test)]
pub(crate) fn vanilla_entity_renderer_thrown_item_scale(entity_type_id: i32) -> Option<f32> {
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

/// Fixed constructor-owned shadow values before living scale/age and renderer-specific overrides.
pub fn vanilla_entity_renderer_base_shadow(entity_type_id: i32) -> (f32, f32) {
    if matches!(
        entity_type_id,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MANNEQUIN_ID
    ) {
        return (
            f32::from_bits(VANILLA_AVATAR_RENDERER_SHADOW_RADIUS_BITS_26_1),
            f32::from_bits(VANILLA_AVATAR_RENDERER_SHADOW_STRENGTH_BITS_26_1),
        );
    }
    vanilla_entity_renderer_registration(entity_type_id)
        .map(|entry| {
            (
                f32::from_bits(entry.shadow_radius_bits),
                f32::from_bits(entry.shadow_strength_bits),
            )
        })
        .unwrap_or((0.0, 1.0))
}

/// Whether the registered vanilla renderer submits through a non-entity-model pass that bbb owns
/// separately (block models, item clusters/frames, firework items, or lightning geometry).
///
/// The renderer classes are stable source identities; membership is still resolved through the
/// generated `EntityRenderers` provider catalog, so a provider change cannot leave a parallel
/// entity-type table stale.
#[cfg(test)]
pub(crate) fn vanilla_entity_renderer_uses_native_non_model_pass(entity_type_id: i32) -> bool {
    matches!(
        vanilla_entity_renderer_presentation_owner(entity_type_id),
        VanillaEntityRendererPresentationOwner::BlockAttachment
            | VanillaEntityRendererPresentationOwner::FireworkItem
            | VanillaEntityRendererPresentationOwner::ItemFrame
            | VanillaEntityRendererPresentationOwner::ItemCluster
            | VanillaEntityRendererPresentationOwner::Lightning
    )
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

fn illager_model_kind(renderer_class: &str) -> Option<VanillaEntityRendererModelKind> {
    use VanillaEntityRendererModelKind as Kind;
    Some(match renderer_class {
        "EvokerRenderer" => Kind::Evoker,
        "IllusionerRenderer" => Kind::Illusioner,
        "PillagerRenderer" => Kind::Pillager,
        "VindicatorRenderer" => Kind::Vindicator,
        _ => return None,
    })
}

fn guardian_model_kind(renderer_class: &str) -> Option<VanillaEntityRendererModelKind> {
    use VanillaEntityRendererModelKind as Kind;
    Some(match renderer_class {
        "GuardianRenderer" => Kind::Guardian,
        "ElderGuardianRenderer" => Kind::ElderGuardian,
        _ => return None,
    })
}

pub(super) fn vanilla_entity_renderer_is_illager(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id)
        .is_some_and(|entry| illager_model_kind(entry.renderer_class).is_some())
}

pub(super) fn vanilla_entity_renderer_is_elder_guardian(entity_type_id: i32) -> bool {
    vanilla_entity_renderer_registration(entity_type_id).is_some_and(|entry| {
        guardian_model_kind(entry.renderer_class)
            == Some(VanillaEntityRendererModelKind::ElderGuardian)
    })
}

pub fn vanilla_entity_renderer_model_kind(
    entity_type_id: i32,
) -> Option<VanillaEntityRendererModelKind> {
    use VanillaEntityRendererModelKind as Kind;
    let entry = vanilla_entity_renderer_registration(entity_type_id)?;
    Some(match entry.renderer_class {
        "SkeletonRenderer" => Kind::Skeleton,
        "StrayRenderer" => Kind::Stray,
        "ParchedRenderer" => Kind::Parched,
        "WitherSkeletonRenderer" => Kind::WitherSkeleton,
        "BoggedRenderer" => Kind::Bogged,
        "CamelRenderer" => Kind::Camel,
        "CamelHuskRenderer" => Kind::CamelHusk,
        "ZombieRenderer" => Kind::Zombie,
        "HuskRenderer" => Kind::Husk,
        "DrownedRenderer" => Kind::Drowned,
        "ZombieVillagerRenderer" => Kind::ZombieVillager,
        "HoglinRenderer" => Kind::Hoglin,
        "ZoglinRenderer" => Kind::Zoglin,
        "SquidRenderer" => Kind::Squid,
        "GlowSquidRenderer" => Kind::GlowSquid,
        _ => {
            if let Some(kind) = illager_model_kind(entry.renderer_class) {
                return Some(kind);
            }
            if let Some(kind) = guardian_model_kind(entry.renderer_class) {
                return Some(kind);
            }
            if let Some(kind) = entry.undead_horse_kind {
                return Some(Kind::UndeadHorse(kind));
            }
            if let Some(kind) = entry.horse_kind {
                return Some(Kind::Horse(kind));
            }
            if let Some(kind) = entry.llama_kind {
                return Some(Kind::Llama(kind));
            }
            if let Some(kind) = entry.boat_kind {
                return Some(Kind::Boat(kind));
            }
            if let Some(kind) = entry.piglin_kind {
                return Some(Kind::Piglin(kind));
            }
            return None;
        }
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use bbb_protocol::entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ALLAY_ID,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
        VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_CAT_ID,
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_EGG_ID,
        VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID, VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
        VANILLA_ENTITY_TYPE_EVOKER_ID, VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
        VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
        VANILLA_ENTITY_TYPE_FIREBALL_ID, VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID,
        VANILLA_ENTITY_TYPE_GIANT_ID, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_GLOW_SQUID_ID, VANILLA_ENTITY_TYPE_GUARDIAN_ID,
        VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_HUSK_ID,
        VANILLA_ENTITY_TYPE_ILLUSIONER_ID, VANILLA_ENTITY_TYPE_INTERACTION_ID,
        VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_ITEM_ID, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
        VANILLA_ENTITY_TYPE_LINGERING_POTION_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        VANILLA_ENTITY_TYPE_MARKER_ID, VANILLA_ENTITY_TYPE_MULE_ID, VANILLA_ENTITY_TYPE_OCELOT_ID,
        VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
        VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARCHED_ID,
        VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID,
        VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_PILLAGER_ID,
        VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
        VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
        VANILLA_ENTITY_TYPE_SKELETON_ID, VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
        VANILLA_ENTITY_TYPE_SNOWBALL_ID, VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
        VANILLA_ENTITY_TYPE_SQUID_ID, VANILLA_ENTITY_TYPE_STRAY_ID,
        VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID, VANILLA_ENTITY_TYPE_TNT_ID,
        VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID, VANILLA_ENTITY_TYPE_TURTLE_ID,
        VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_VILLAGER_ID,
        VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
        VANILLA_ENTITY_TYPE_WITCH_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOGLIN_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
    };

    #[test]
    fn generated_renderer_classes_drive_visibility_culling_exceptions() {
        assert_eq!(
            vanilla_entity_renderer_culling_kind(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID
            ),
            VanillaEntityRendererCullingKind::NoFrustum
        );
        assert_eq!(
            vanilla_entity_renderer_culling_kind(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SNIFFER_ID
            ),
            VanillaEntityRendererCullingKind::Inflate([0.6; 3])
        );
        assert_eq!(
            vanilla_entity_renderer_culling_kind(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ILLUSIONER_ID
            ),
            VanillaEntityRendererCullingKind::Inflate([3.0, 0.0, 3.0])
        );
        assert_eq!(
            vanilla_entity_renderer_culling_kind(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID
            ),
            VanillaEntityRendererCullingKind::ExtendMinYByHalfHeight
        );
        for entity_type_id in [
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GUARDIAN_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_END_CRYSTAL_ID,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID,
        ] {
            assert_eq!(
                vanilla_entity_renderer_culling_kind(entity_type_id),
                VanillaEntityRendererCullingKind::ConservativeSpecial
            );
        }
        assert_eq!(
            vanilla_entity_renderer_culling_kind(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID
            ),
            VanillaEntityRendererCullingKind::Display
        );
        assert_eq!(
            vanilla_entity_renderer_culling_kind(
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID
            ),
            VanillaEntityRendererCullingKind::Standard
        );
    }

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
    fn presentation_owner_exhaustively_partitions_every_fixed_entity_type() {
        use VanillaEntityRendererPresentationOwner as Owner;

        let ids_for = |owner| {
            VANILLA_ENTITY_TYPES
                .iter()
                .filter_map(|entry| {
                    (vanilla_entity_renderer_presentation_owner(entry.id) == owner)
                        .then_some(entry.id)
                })
                .collect::<Vec<_>>()
        };

        assert_eq!(
            ids_for(Owner::AvatarModel),
            vec![
                VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
                VANILLA_ENTITY_TYPE_PLAYER_ID,
            ]
        );
        assert_eq!(ids_for(Owner::EntityModel).len(), 131);
        assert_eq!(
            ids_for(Owner::Noop),
            vec![
                VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
                VANILLA_ENTITY_TYPE_INTERACTION_ID,
                VANILLA_ENTITY_TYPE_MARKER_ID,
            ]
        );
        assert_eq!(
            ids_for(Owner::ThrownItem),
            vec![
                VANILLA_ENTITY_TYPE_EGG_ID,
                VANILLA_ENTITY_TYPE_ENDER_PEARL_ID,
                VANILLA_ENTITY_TYPE_EXPERIENCE_BOTTLE_ID,
                VANILLA_ENTITY_TYPE_EYE_OF_ENDER_ID,
                VANILLA_ENTITY_TYPE_FIREBALL_ID,
                VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
                VANILLA_ENTITY_TYPE_LINGERING_POTION_ID,
                VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
                VANILLA_ENTITY_TYPE_SNOWBALL_ID,
            ]
        );
        assert_eq!(
            ids_for(Owner::BlockAttachment),
            vec![
                VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
                VANILLA_ENTITY_TYPE_TNT_ID
            ]
        );
        assert_eq!(
            ids_for(Owner::FireworkItem),
            vec![VANILLA_ENTITY_TYPE_FIREWORK_ROCKET_ID]
        );
        assert_eq!(
            ids_for(Owner::ItemFrame),
            vec![
                VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
                VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            ]
        );
        assert_eq!(
            ids_for(Owner::ItemCluster),
            vec![
                VANILLA_ENTITY_TYPE_ITEM_ID,
                VANILLA_ENTITY_TYPE_OMINOUS_ITEM_SPAWNER_ID,
            ]
        );
        assert_eq!(
            ids_for(Owner::Lightning),
            vec![VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID]
        );
        assert_eq!(
            ids_for(Owner::Display),
            vec![
                VANILLA_ENTITY_TYPE_BLOCK_DISPLAY_ID,
                VANILLA_ENTITY_TYPE_ITEM_DISPLAY_ID,
                VANILLA_ENTITY_TYPE_TEXT_DISPLAY_ID,
            ]
        );
        assert_eq!(
            ids_for(Owner::Painting),
            vec![VANILLA_ENTITY_TYPE_PAINTING_ID]
        );
        assert!(ids_for(Owner::Unknown).is_empty());
        assert_eq!(
            vanilla_entity_renderer_presentation_owner(-1),
            Owner::Unknown
        );
        assert_eq!(
            vanilla_entity_renderer_presentation_owner(VANILLA_ENTITY_TYPES.len() as i32),
            Owner::Unknown
        );
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
            assert_eq!(
                vanilla_entity_renderer_block_light_floor(entity_type_id),
                15
            );
        }
        assert_eq!(vanilla_entity_renderer_block_light_floor(-1), 0);
        assert_eq!(vanilla_entity_renderer_block_light_floor(157), 0);
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
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_SKELETON_ID),
            Some(VanillaEntityRendererModelKind::Skeleton)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_STRAY_ID),
            Some(VanillaEntityRendererModelKind::Stray)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_PARCHED_ID),
            Some(VanillaEntityRendererModelKind::Parched)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID),
            Some(VanillaEntityRendererModelKind::WitherSkeleton)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_BOGGED_ID),
            Some(VanillaEntityRendererModelKind::Bogged)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            Some(VanillaEntityRendererModelKind::Zombie)
        );
    }

    #[test]
    fn generated_illager_kinds_match_registered_renderer_classes() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_EVOKER_ID),
            Some(VanillaEntityRendererModelKind::Evoker)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ILLUSIONER_ID),
            Some(VanillaEntityRendererModelKind::Illusioner)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_PILLAGER_ID),
            Some(VanillaEntityRendererModelKind::Pillager)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_VINDICATOR_ID),
            Some(VanillaEntityRendererModelKind::Vindicator)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_WITCH_ID),
            None
        );
    }

    #[test]
    fn generated_camel_kinds_match_registered_renderer_classes() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_CAMEL_ID),
            Some(VanillaEntityRendererModelKind::Camel)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
            Some(VanillaEntityRendererModelKind::CamelHusk)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID),
            Some(VanillaEntityRendererModelKind::Horse(
                VanillaHorseRendererKind::Horse
            ))
        );
    }

    #[test]
    fn generated_undead_horse_kinds_match_provider_constructor_arguments() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID),
            Some(VanillaEntityRendererModelKind::UndeadHorse(
                UndeadHorseModelFamily::Skeleton
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID),
            Some(VanillaEntityRendererModelKind::UndeadHorse(
                UndeadHorseModelFamily::Zombie
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID),
            Some(VanillaEntityRendererModelKind::Horse(
                VanillaHorseRendererKind::Horse
            ))
        );
    }

    #[test]
    fn generated_horse_kinds_match_provider_classes_and_constructor_arguments() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID),
            Some(VanillaEntityRendererModelKind::Horse(
                VanillaHorseRendererKind::Horse
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_DONKEY_ID),
            Some(VanillaEntityRendererModelKind::Horse(
                VanillaHorseRendererKind::Donkey
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_MULE_ID),
            Some(VanillaEntityRendererModelKind::Horse(
                VanillaHorseRendererKind::Mule
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID),
            Some(VanillaEntityRendererModelKind::UndeadHorse(
                UndeadHorseModelFamily::Skeleton
            ))
        );
    }

    #[test]
    fn generated_llama_kinds_match_provider_constructor_layers() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_LLAMA_ID),
            Some(VanillaEntityRendererModelKind::Llama(
                LlamaModelFamily::Llama
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID),
            Some(VanillaEntityRendererModelKind::Llama(
                LlamaModelFamily::TraderLlama
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_HORSE_ID),
            Some(VanillaEntityRendererModelKind::Horse(
                VanillaHorseRendererKind::Horse
            ))
        );
    }

    #[test]
    fn generated_boat_kinds_match_provider_classes_and_model_layers() {
        let actual = VANILLA_ENTITY_RENDERER_REGISTRATIONS_26_1
            .iter()
            .filter_map(
                |entry| match vanilla_entity_renderer_model_kind(entry.entity_type_id)? {
                    VanillaEntityRendererModelKind::Boat(kind) => {
                        Some((entry.constant_name, kind.family, kind.chest))
                    }
                    _ => None,
                },
            )
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            vec![
                ("ACACIA_BOAT", BoatModelFamily::Acacia, false),
                ("ACACIA_CHEST_BOAT", BoatModelFamily::Acacia, true),
                ("BAMBOO_CHEST_RAFT", BoatModelFamily::Bamboo, true),
                ("BAMBOO_RAFT", BoatModelFamily::Bamboo, false),
                ("BIRCH_BOAT", BoatModelFamily::Birch, false),
                ("BIRCH_CHEST_BOAT", BoatModelFamily::Birch, true),
                ("CHERRY_BOAT", BoatModelFamily::Cherry, false),
                ("CHERRY_CHEST_BOAT", BoatModelFamily::Cherry, true),
                ("DARK_OAK_BOAT", BoatModelFamily::DarkOak, false),
                ("DARK_OAK_CHEST_BOAT", BoatModelFamily::DarkOak, true),
                ("JUNGLE_BOAT", BoatModelFamily::Jungle, false),
                ("JUNGLE_CHEST_BOAT", BoatModelFamily::Jungle, true),
                ("MANGROVE_BOAT", BoatModelFamily::Mangrove, false),
                ("MANGROVE_CHEST_BOAT", BoatModelFamily::Mangrove, true),
                ("OAK_BOAT", BoatModelFamily::Oak, false),
                ("OAK_CHEST_BOAT", BoatModelFamily::Oak, true),
                ("PALE_OAK_BOAT", BoatModelFamily::PaleOak, false),
                ("PALE_OAK_CHEST_BOAT", BoatModelFamily::PaleOak, true),
                ("SPRUCE_BOAT", BoatModelFamily::Spruce, false),
                ("SPRUCE_CHEST_BOAT", BoatModelFamily::Spruce, true),
            ]
        );
    }

    #[test]
    fn generated_zombie_kinds_match_provider_classes() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            Some(VanillaEntityRendererModelKind::Zombie)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_HUSK_ID),
            Some(VanillaEntityRendererModelKind::Husk)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_DROWNED_ID),
            Some(VanillaEntityRendererModelKind::Drowned)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID),
            Some(VanillaEntityRendererModelKind::ZombieVillager)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID),
            Some(VanillaEntityRendererModelKind::Piglin(
                PiglinModelFamily::ZombifiedPiglin
            ))
        );
    }

    #[test]
    fn generated_piglin_kinds_match_provider_constructor_layers() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_ID),
            Some(VanillaEntityRendererModelKind::Piglin(
                PiglinModelFamily::Piglin
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_BRUTE_ID),
            Some(VanillaEntityRendererModelKind::Piglin(
                PiglinModelFamily::PiglinBrute
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID),
            Some(VanillaEntityRendererModelKind::Piglin(
                PiglinModelFamily::ZombifiedPiglin
            ))
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOMBIE_ID),
            Some(VanillaEntityRendererModelKind::Zombie)
        );
    }

    #[test]
    fn generated_hoglin_kinds_match_provider_classes() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_HOGLIN_ID),
            Some(VanillaEntityRendererModelKind::Hoglin)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ZOGLIN_ID),
            Some(VanillaEntityRendererModelKind::Zoglin)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_PIGLIN_ID),
            Some(VanillaEntityRendererModelKind::Piglin(
                PiglinModelFamily::Piglin
            ))
        );
    }

    #[test]
    fn generated_guardian_kinds_match_provider_classes() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_GUARDIAN_ID),
            Some(VanillaEntityRendererModelKind::Guardian)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_ELDER_GUARDIAN_ID),
            Some(VanillaEntityRendererModelKind::ElderGuardian)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_DOLPHIN_ID),
            None
        );
    }

    #[test]
    fn generated_squid_kinds_match_provider_classes() {
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_SQUID_ID),
            Some(VanillaEntityRendererModelKind::Squid)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_GLOW_SQUID_ID),
            Some(VanillaEntityRendererModelKind::GlowSquid)
        );
        assert_eq!(
            vanilla_entity_renderer_model_kind(VANILLA_ENTITY_TYPE_GUARDIAN_ID),
            Some(VanillaEntityRendererModelKind::Guardian)
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
    fn generated_shadow_values_match_renderer_constructor_paths() {
        assert_eq!(
            vanilla_entity_renderer_base_shadow(VANILLA_ENTITY_TYPE_GIANT_ID),
            (3.0, 1.0)
        );
        assert_eq!(
            vanilla_entity_renderer_base_shadow(VANILLA_ENTITY_TYPE_ITEM_ID),
            (0.15, 0.75)
        );
        for entity_type_id in [
            VANILLA_ENTITY_TYPE_PLAYER_ID,
            VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        ] {
            assert_eq!(
                vanilla_entity_renderer_base_shadow(entity_type_id),
                (0.5, 1.0)
            );
        }
        assert_eq!(
            vanilla_entity_renderer_base_shadow(VANILLA_ENTITY_TYPE_MARKER_ID),
            (0.0, 1.0)
        );
        assert_eq!(vanilla_entity_renderer_base_shadow(-1), (0.0, 1.0));
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
