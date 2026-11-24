//! Holder-id order for the registries a vanilla server builds out of data files.
//!
//! Two different orders decide what a holder id means, and the client has to
//! use the right one whenever no `ClientboundRegistryDataPacket` supplied the
//! registry.
//!
//! * `BuiltInRegistries` registries are filled by a bootstrap method, so holder
//!   ids follow the order the constants are registered in
//!   (BuiltInRegistries.java:176-210, BuiltInRegistries.java:298,
//!   BuiltInRegistries.java:303-305). Those stay on the checked-in bootstrap
//!   catalogs and are not routed through this module.
//! * The registries in `RegistryDataLoader.WORLDGEN_REGISTRIES`
//!   (RegistryDataLoader.java:83-129) and
//!   `RegistryDataLoader.SYNCHRONIZED_REGISTRIES`
//!   (RegistryDataLoader.java:133-162) are filled from
//!   `data/<namespace>/<registry>/*.json`. The loader sorts the listed
//!   resources by `Identifier` before registering them
//!   (ResourceManagerRegistryLoadTask.java:62, RegistryLoadTask.java:62-70),
//!   and `Identifier` compares path first and namespace second
//!   (Identifier.java:142-149). For an all-vanilla server that is plain
//!   lexicographic order over the element paths, so holder ids follow sorted
//!   resource ids, not the bootstrap declaration order.
//!
//! The observed value that forced this split: the official 26.1 server reports
//! `minecraft:out_of_world` as source type 32 in `ClientboundDamageEventPacket`
//! (DamageType.java:24 encodes the holder through the synchronized registry),
//! and 32 is exactly its index once the 50 checked-in damage types are sorted
//! by resource id. Its bootstrap index is 14.
//!
//! The sorted order is derived here rather than written down a second time:
//! every entry below reads the existing bootstrap catalog through its own
//! accessor and re-sorts it. Each catalog's member set was checked against
//! `data/minecraft/<registry>/` in vanilla 26.1 and matches exactly; a registry
//! whose members could not be enumerated unambiguously resolves to `None`
//! instead of guessing.
//!
//! This module is the only owner of that order. It is exported from the crate
//! so `bbb-client-render` and `bbb-client` resolve unsynchronized holder ids
//! through it instead of keeping their own copies, and
//! `no_other_production_code_resolves_a_loader_ordered_registry_by_bootstrap_index`
//! fails the build if a second copy reappears. The catalogs it derives from are
//! split across `bbb-protocol` (damage type, painting variant, chat type, world
//! clock, enchantment, animal sound variants) and `bbb-render-types` (banner
//! pattern and the animal model variants), and `bbb-protocol` does not depend on
//! `bbb-render-types`, so `bbb-client-world` is the lowest crate that can see all of
//! them.

use std::sync::OnceLock;

use bbb_protocol::damage_types::vanilla_damage_type;
use bbb_protocol::enchantment_types::VanillaEnchantmentIdentity;
use bbb_protocol::painting_variant_types::vanilla_painting_variant;
use bbb_protocol::registry_data_types::VanillaChatTypeIdentity;
use bbb_protocol::world_clock_types::vanilla_world_clock_type_by_id;
use bbb_render_types::{
    cat_variants::CatModelVariant,
    temperature_animal_variants::{
        ChickenModelVariant, CowModelVariant, FrogModelVariant, PigModelVariant,
    },
    wolf_variants::WolfModelVariant,
    zombie_nautilus_variants::ZombieNautilusModelVariant,
    BannerPatternKind,
};

use crate::{
    CatSoundVariantIdentity, ChickenSoundVariantIdentity, CowSoundVariantIdentity,
    PigSoundVariantIdentity, WolfSoundVariantIdentity,
};

use super::instruments::VanillaInstrumentIdentity;
use super::trim_materials::VanillaTrimMaterialIdentity;
use super::trim_patterns::VanillaTrimPatternIdentity;

/// Reads one resource id out of a checked-in bootstrap catalog by its
/// bootstrap index, or `None` once the catalog runs out.
type BootstrapProbe = fn(i32) -> Option<&'static str>;

/// The registries whose fallback holder ids are loader order, each paired with
/// the bootstrap catalog the member set is enumerated from.
///
/// Every registry here appears in `RegistryDataLoader.WORLDGEN_REGISTRIES`
/// (RegistryDataLoader.java:83-129); the line beside each entry is its row in
/// that list.
const LOADER_ORDERED_REGISTRIES: &[(&str, BootstrapProbe)] = &[
    // RegistryDataLoader.java:115
    ("minecraft:damage_type", damage_type_bootstrap_entry),
    // RegistryDataLoader.java:86
    ("minecraft:chat_type", chat_type_bootstrap_entry),
    // RegistryDataLoader.java:107
    ("minecraft:cat_variant", cat_variant_bootstrap_entry),
    // RegistryDataLoader.java:108
    (
        "minecraft:cat_sound_variant",
        cat_sound_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:109
    ("minecraft:cow_variant", cow_variant_bootstrap_entry),
    // RegistryDataLoader.java:110
    (
        "minecraft:cow_sound_variant",
        cow_sound_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:102
    ("minecraft:wolf_variant", wolf_variant_bootstrap_entry),
    // RegistryDataLoader.java:103
    (
        "minecraft:wolf_sound_variant",
        wolf_sound_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:106
    ("minecraft:frog_variant", frog_variant_bootstrap_entry),
    // RegistryDataLoader.java:104
    ("minecraft:pig_variant", pig_variant_bootstrap_entry),
    // RegistryDataLoader.java:105
    (
        "minecraft:pig_sound_variant",
        pig_sound_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:111
    ("minecraft:chicken_variant", chicken_variant_bootstrap_entry),
    // RegistryDataLoader.java:112
    (
        "minecraft:chicken_sound_variant",
        chicken_sound_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:113
    (
        "minecraft:zombie_nautilus_variant",
        zombie_nautilus_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:114
    (
        "minecraft:painting_variant",
        painting_variant_bootstrap_entry,
    ),
    // RegistryDataLoader.java:125
    ("minecraft:world_clock", world_clock_bootstrap_entry),
    // RegistryDataLoader.java:117
    ("minecraft:banner_pattern", banner_pattern_bootstrap_entry),
    // RegistryDataLoader.java:118
    ("minecraft:enchantment", enchantment_bootstrap_entry),
    // RegistryDataLoader.java:121
    ("minecraft:instrument", instrument_bootstrap_entry),
    // RegistryDataLoader.java:99
    ("minecraft:trim_pattern", trim_pattern_bootstrap_entry),
    // RegistryDataLoader.java:100
    ("minecraft:trim_material", trim_material_bootstrap_entry),
];

/// One registry's resource ids in the order `RegistryDataLoader` registers
/// them, which is the order the holder ids on the wire count in.
struct LoaderOrder {
    registry: &'static str,
    resource_ids: Vec<&'static str>,
}

/// True when this registry's fallback holder ids are loader order.
///
/// Callers use it to keep a registry off the bootstrap catalogs even when the
/// order could not be derived, so an unknown holder id resolves to nothing
/// rather than to the bootstrap entry that used to sit at that index.
pub fn is_loader_ordered_registry(registry: &str) -> bool {
    LOADER_ORDERED_REGISTRIES
        .iter()
        .any(|(name, _)| *name == registry)
}

/// The resource id a vanilla server would give this holder id, or `None` when
/// the registry is not loader-ordered or the id is past the end of it.
pub fn loader_ordered_resource_id(registry: &str, holder_id: i32) -> Option<&'static str> {
    let index = usize::try_from(holder_id).ok()?;
    loader_order(registry)?.resource_ids.get(index).copied()
}

/// The holder id a vanilla server would give this resource id, or `None` when
/// the registry is not loader-ordered or the resource is not a vanilla member.
pub fn loader_ordered_holder_id(registry: &str, resource_id: &str) -> Option<i32> {
    let resource_ids = &loader_order(registry)?.resource_ids;
    let index = resource_ids.binary_search(&resource_id).ok()?;
    i32::try_from(index).ok()
}

pub(crate) fn loader_ordered_registry_available(registry: &str) -> bool {
    loader_order(registry).is_some()
}

fn loader_order(registry: &str) -> Option<&'static LoaderOrder> {
    static ORDERS: OnceLock<Vec<LoaderOrder>> = OnceLock::new();
    ORDERS
        .get_or_init(build_loader_orders)
        .iter()
        .find(|order| order.registry == registry)
}

fn build_loader_orders() -> Vec<LoaderOrder> {
    LOADER_ORDERED_REGISTRIES
        .iter()
        .filter_map(|(registry, probe)| {
            Some(LoaderOrder {
                registry,
                resource_ids: sorted_catalog_members(*probe)?,
            })
        })
        .collect()
}

/// Walks a bootstrap catalog from index zero and returns its members sorted by
/// resource id.
///
/// An empty catalog or a duplicated resource id means the member set cannot be
/// trusted to be the one the loader would list, so nothing is returned and the
/// registry stays unresolved: a wrong holder id is worse than a missing one.
fn sorted_catalog_members(probe: BootstrapProbe) -> Option<Vec<&'static str>> {
    let mut resource_ids: Vec<&'static str> = (0..).map_while(probe).collect();
    if resource_ids.is_empty() {
        return None;
    }
    resource_ids.sort_unstable();
    if resource_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return None;
    }
    Some(resource_ids)
}

fn damage_type_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(vanilla_damage_type(holder_id)?.resource_id)
}

fn chat_type_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(VanillaChatTypeIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn cat_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(CatModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn cat_sound_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(CatSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn cow_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(CowModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn cow_sound_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(CowSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn wolf_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(WolfModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn wolf_sound_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(WolfSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn frog_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(FrogModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn pig_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(PigModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn pig_sound_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(PigSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn chicken_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(ChickenModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn chicken_sound_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(ChickenSoundVariantIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn zombie_nautilus_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(ZombieNautilusModelVariant::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn painting_variant_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(vanilla_painting_variant(holder_id)?.resource_id)
}

fn world_clock_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(vanilla_world_clock_type_by_id(holder_id)?.resource_id)
}

fn banner_pattern_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(BannerPatternKind::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn enchantment_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(VanillaEnchantmentIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn instrument_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(VanillaInstrumentIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn trim_pattern_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(VanillaTrimPatternIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

fn trim_material_bootstrap_entry(holder_id: i32) -> Option<&'static str> {
    Some(VanillaTrimMaterialIdentity::for_vanilla_registry_id(holder_id)?.resource_id())
}

#[cfg(test)]
mod tests {
    use bbb_protocol::damage_types::vanilla_damage_type_for_resource_id;
    use bbb_protocol::world_clock_types::{
        VANILLA_WORLD_CLOCK_OVERWORLD_ID, VANILLA_WORLD_CLOCK_THE_END_ID,
    };

    use super::*;

    /// One loader-ordered registry with the two heads it has in each order and
    /// whether sorting moves anything at all.
    ///
    /// `reorders == false` records a coincidence, not a licence: those
    /// registries are data-loaded like the rest, their bootstrap declaration
    /// order simply happens to be alphabetical in vanilla 26.1. The assertion
    /// exists so a vanilla change that breaks the coincidence is caught here
    /// rather than in a silent misdecode.
    struct OrderExpectation {
        registry: &'static str,
        loader_head: &'static str,
        bootstrap_head: &'static str,
        reorders: bool,
        members: usize,
    }

    const EXPECTATIONS: &[OrderExpectation] = &[
        expectation(
            "minecraft:damage_type",
            "minecraft:arrow",
            "minecraft:in_fire",
            true,
            50,
        ),
        expectation(
            "minecraft:chat_type",
            "minecraft:chat",
            "minecraft:chat",
            true,
            7,
        ),
        expectation(
            "minecraft:cat_variant",
            "minecraft:all_black",
            "minecraft:tabby",
            true,
            11,
        ),
        expectation(
            "minecraft:cat_sound_variant",
            "minecraft:classic",
            "minecraft:classic",
            false,
            2,
        ),
        expectation(
            "minecraft:cow_variant",
            "minecraft:cold",
            "minecraft:temperate",
            true,
            3,
        ),
        expectation(
            "minecraft:cow_sound_variant",
            "minecraft:classic",
            "minecraft:classic",
            false,
            2,
        ),
        expectation(
            "minecraft:wolf_variant",
            "minecraft:ashen",
            "minecraft:pale",
            true,
            9,
        ),
        expectation(
            "minecraft:wolf_sound_variant",
            "minecraft:angry",
            "minecraft:classic",
            true,
            7,
        ),
        expectation(
            "minecraft:frog_variant",
            "minecraft:cold",
            "minecraft:temperate",
            true,
            3,
        ),
        expectation(
            "minecraft:pig_variant",
            "minecraft:cold",
            "minecraft:temperate",
            true,
            3,
        ),
        expectation(
            "minecraft:pig_sound_variant",
            "minecraft:big",
            "minecraft:classic",
            true,
            3,
        ),
        expectation(
            "minecraft:chicken_variant",
            "minecraft:cold",
            "minecraft:temperate",
            true,
            3,
        ),
        expectation(
            "minecraft:chicken_sound_variant",
            "minecraft:classic",
            "minecraft:classic",
            false,
            2,
        ),
        expectation(
            "minecraft:zombie_nautilus_variant",
            "minecraft:temperate",
            "minecraft:temperate",
            false,
            2,
        ),
        expectation(
            "minecraft:painting_variant",
            "minecraft:alban",
            "minecraft:kebab",
            true,
            51,
        ),
        expectation(
            "minecraft:world_clock",
            "minecraft:overworld",
            "minecraft:overworld",
            false,
            2,
        ),
        expectation(
            "minecraft:banner_pattern",
            "minecraft:base",
            "minecraft:base",
            true,
            43,
        ),
        expectation(
            "minecraft:enchantment",
            "minecraft:aqua_affinity",
            "minecraft:protection",
            true,
            43,
        ),
        expectation(
            "minecraft:instrument",
            "minecraft:admire_goat_horn",
            "minecraft:ponder_goat_horn",
            true,
            8,
        ),
        expectation(
            "minecraft:trim_pattern",
            "minecraft:bolt",
            "minecraft:sentry",
            true,
            18,
        ),
        expectation(
            "minecraft:trim_material",
            "minecraft:amethyst",
            "minecraft:quartz",
            true,
            11,
        ),
    ];

    const fn expectation(
        registry: &'static str,
        loader_head: &'static str,
        bootstrap_head: &'static str,
        reorders: bool,
        members: usize,
    ) -> OrderExpectation {
        OrderExpectation {
            registry,
            loader_head,
            bootstrap_head,
            reorders,
            members,
        }
    }

    fn bootstrap_members(registry: &str) -> Vec<&'static str> {
        let probe = LOADER_ORDERED_REGISTRIES
            .iter()
            .find(|(name, _)| *name == registry)
            .map(|(_, probe)| *probe)
            .expect("loader-ordered registry");
        (0..).map_while(probe).collect()
    }

    /// The official 26.1 server answers `minecraft:out_of_world` with source
    /// type 32 in `ClientboundDamageEventPacket`
    /// (ClientboundDamageEventPacket.java:16, DamageType.java:24), which is its
    /// index once the damage types are sorted by resource id. 14 is its
    /// bootstrap index and the value this fallback used to return.
    #[test]
    fn out_of_world_damage_type_falls_back_to_the_official_thirty_two() {
        assert_eq!(
            loader_ordered_holder_id("minecraft:damage_type", "minecraft:out_of_world"),
            Some(32)
        );
        assert_eq!(
            loader_ordered_resource_id("minecraft:damage_type", 32),
            Some("minecraft:out_of_world")
        );
        assert_eq!(
            vanilla_damage_type_for_resource_id("minecraft:out_of_world").map(|entry| entry.id),
            Some(14),
            "the bootstrap catalog still carries the declaration order"
        );
        assert_eq!(
            loader_ordered_resource_id("minecraft:damage_type", 14),
            Some("minecraft:fireball"),
            "holder 14 is no longer out_of_world"
        );
    }

    /// Every registry this module claims is loader-ordered must resolve both
    /// ways, agree with itself, and stop at the end of the catalog.
    #[test]
    fn loader_order_is_sorted_dense_and_invertible_in_both_directions() {
        for (registry, _) in LOADER_ORDERED_REGISTRIES {
            assert!(is_loader_ordered_registry(registry));
            let members = bootstrap_members(registry);
            let mut expected = members.clone();
            expected.sort_unstable();
            for (holder_id, resource_id) in expected.iter().enumerate() {
                let holder_id = i32::try_from(holder_id).unwrap();
                assert_eq!(
                    loader_ordered_resource_id(registry, holder_id),
                    Some(*resource_id),
                    "{registry} holder {holder_id}"
                );
                assert_eq!(
                    loader_ordered_holder_id(registry, resource_id),
                    Some(holder_id),
                    "{registry} resource {resource_id}"
                );
            }
            let past_end = i32::try_from(expected.len()).unwrap();
            assert_eq!(loader_ordered_resource_id(registry, past_end), None);
            assert_eq!(loader_ordered_resource_id(registry, -1), None);
            assert_eq!(loader_ordered_holder_id(registry, "minecraft:absent"), None);
        }
    }

    /// The whole point of the split: for most of these registries the sorted
    /// order really is a different order from the bootstrap declaration order,
    /// so answering with the bootstrap index would hand back the wrong entry.
    #[test]
    fn loader_order_differs_from_bootstrap_order_exactly_where_vanilla_declares_it_out_of_sequence()
    {
        assert_eq!(EXPECTATIONS.len(), LOADER_ORDERED_REGISTRIES.len());
        for expectation in EXPECTATIONS {
            let registry = expectation.registry;
            let members = bootstrap_members(registry);
            assert_eq!(
                members.len(),
                expectation.members,
                "{registry} member count"
            );
            let mut sorted = members.clone();
            sorted.sort_unstable();
            assert_eq!(
                members != sorted,
                expectation.reorders,
                "{registry} loader order versus bootstrap order"
            );
            assert_eq!(members.first().copied(), Some(expectation.bootstrap_head));
            assert_eq!(
                loader_ordered_resource_id(registry, 0),
                Some(expectation.loader_head),
                "{registry} holder 0"
            );
        }
        assert!(
            EXPECTATIONS.iter().any(|expectation| expectation.reorders),
            "a tree where no registry reorders would make this module pointless"
        );
    }

    /// `minecraft:world_clock` is data-loaded (RegistryDataLoader.java:125), so
    /// the two named bootstrap constants are only still correct because vanilla
    /// happens to declare `overworld` before `the_end` and that is also their
    /// alphabetical order. Adding a world clock that sorts before `overworld`
    /// would move both, so the coincidence is pinned rather than assumed.
    #[test]
    fn world_clock_bootstrap_constants_still_match_loader_order() {
        assert_eq!(
            loader_ordered_holder_id("minecraft:world_clock", "minecraft:overworld"),
            Some(VANILLA_WORLD_CLOCK_OVERWORLD_ID)
        );
        assert_eq!(
            loader_ordered_holder_id("minecraft:world_clock", "minecraft:the_end"),
            Some(VANILLA_WORLD_CLOCK_THE_END_ID)
        );
        assert_eq!(
            loader_ordered_resource_id("minecraft:world_clock", VANILLA_WORLD_CLOCK_OVERWORLD_ID),
            Some("minecraft:overworld")
        );
        assert_eq!(
            loader_ordered_resource_id("minecraft:world_clock", VANILLA_WORLD_CLOCK_THE_END_ID),
            Some("minecraft:the_end")
        );
    }

    /// `BuiltInRegistries` registries keep bootstrap order and must never be
    /// routed through here (BuiltInRegistries.java:176-210).
    #[test]
    fn built_in_registries_are_not_loader_ordered() {
        for registry in [
            "minecraft:attribute",
            "minecraft:item",
            "minecraft:block",
            "minecraft:fluid",
            "minecraft:entity_type",
            "minecraft:game_event",
            "minecraft:mob_effect",
            "minecraft:potion",
            "minecraft:sound_event",
            "minecraft:stat_type",
            "minecraft:custom_stat",
            "minecraft:particle_type",
            "minecraft:data_component_type",
            "minecraft:data_component_predicate_type",
            "minecraft:villager_type",
            "minecraft:villager_profession",
        ] {
            assert!(!is_loader_ordered_registry(registry), "{registry}");
            assert_eq!(loader_ordered_resource_id(registry, 0), None);
            assert_eq!(loader_ordered_holder_id(registry, "minecraft:air"), None);
        }
    }
}
