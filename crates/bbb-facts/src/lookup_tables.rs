//! Lazy open-addressed FNV-1a index tables for static resource-id catalogs.

use std::sync::OnceLock;

use crate::{
    attribute_types::{AttributeTypeCatalogEntry, VANILLA_ATTRIBUTE_TYPES},
    block_entity_types::{BlockEntityTypeCatalogEntry, VANILLA_BLOCK_ENTITY_TYPES},
    block_sound_facts_26_1::{vanilla_block_sound_group_26_1, VanillaBlockSoundGroupFacts26_1},
    block_types::{BlockTypeCatalogEntry, VANILLA_BLOCK_TYPES},
    data_component_types::{DataComponentTypeCatalogEntry, VANILLA_DATA_COMPONENT_TYPES},
    entity_types::{EntityTypeCatalogEntry, VANILLA_ENTITY_TYPES},
    item_types::{ItemTypeCatalogEntry, VANILLA_ITEM_TYPES},
    map_decorations::{VanillaMapDecorationType, MAP_DECORATION_CATALOG},
    mob_effect_types::{MobEffectTypeCatalogEntry, VANILLA_MOB_EFFECT_TYPES},
    particle_types::{ParticleTypeCatalogEntry, VANILLA_PARTICLE_TYPES},
    registry_data_types::{BuiltInRegistryType, VANILLA_BUILT_IN_REGISTRY_TYPES},
    sound_event_types::VANILLA_SOUND_EVENT_RESOURCE_IDS,
    stat_types::{
        CustomStatCatalogEntry, StatTypeCatalogEntry, VANILLA_CUSTOM_STATS, VANILLA_STAT_TYPES,
    },
    static_registry_elements::VANILLA_STATIC_REGISTRY_ELEMENTS,
};

const EMPTY_SLOT: u16 = u16::MAX;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0100_0000_01b3;

const fn fnv1a_bytes(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

const fn strip_minecraft_prefix(resource_id: &str) -> Option<&[u8]> {
    let bytes = resource_id.as_bytes();
    let prefix = b"minecraft:";
    if bytes.len() <= prefix.len() {
        return None;
    }
    let mut i = 0;
    while i < prefix.len() {
        if bytes[i] != prefix[i] {
            return None;
        }
        i += 1;
    }
    Some(bytes.split_at(prefix.len()).1)
}

#[inline]
fn default_minecraft_path(resource_id: &str) -> Option<&[u8]> {
    let path = resource_id
        .strip_prefix("minecraft:")
        .unwrap_or(resource_id);
    if path.is_empty() || path.as_bytes().contains(&b':') {
        return None;
    }
    Some(path.as_bytes())
}

macro_rules! define_exact_entry_table {
    ($table_fn:ident, $cap:expr, $slice:ident, $field:ident) => {
        const _: () = {
            assert!(($cap as usize).is_power_of_two());
            assert!($slice.len() * 2 <= $cap);
        };

        #[inline]
        fn $table_fn() -> &'static [u16; $cap] {
            static TABLE: OnceLock<[u16; $cap]> = OnceLock::new();
            TABLE.get_or_init(|| {
                let mut table = [EMPTY_SLOT; $cap];
                let mask = $cap - 1;
                for (idx, entry) in $slice.iter().enumerate() {
                    let key = entry.$field.as_bytes();
                    let mut slot = (fnv1a_bytes(key) as usize) & mask;
                    loop {
                        let existing = table[slot];
                        if existing == EMPTY_SLOT {
                            table[slot] = idx as u16;
                            break;
                        }
                        if $slice[existing as usize].$field.as_bytes() == key {
                            break;
                        }
                        slot = (slot + 1) & mask;
                    }
                }
                table
            })
        }
    };
}

macro_rules! define_minecraft_path_entry_table {
    ($table_fn:ident, $cap:expr, $slice:ident, $field:ident) => {
        const _: () = {
            assert!(($cap as usize).is_power_of_two());
            assert!($slice.len() * 2 <= $cap);
        };

        #[inline]
        fn $table_fn() -> &'static [u16; $cap] {
            static TABLE: OnceLock<[u16; $cap]> = OnceLock::new();
            TABLE.get_or_init(|| {
                let mut table = [EMPTY_SLOT; $cap];
                let mask = $cap - 1;
                for (idx, entry) in $slice.iter().enumerate() {
                    if let Some(key) = strip_minecraft_prefix(entry.$field) {
                        let mut slot = (fnv1a_bytes(key) as usize) & mask;
                        loop {
                            let existing = table[slot];
                            if existing == EMPTY_SLOT {
                                table[slot] = idx as u16;
                                break;
                            }
                            if strip_minecraft_prefix($slice[existing as usize].$field) == Some(key)
                            {
                                break;
                            }
                            slot = (slot + 1) & mask;
                        }
                    }
                }
                table
            })
        }
    };
}

macro_rules! lookup_exact_entry {
    ($table_fn:ident, $slice:ident, $field:ident, $key:expr) => {{
        let table = $table_fn();
        let key_bytes = $key.as_bytes();
        let mask = table.len() - 1;
        let mut slot = (fnv1a_bytes(key_bytes) as usize) & mask;
        loop {
            let idx = table[slot];
            if idx == EMPTY_SLOT {
                break None;
            }
            let entry = &$slice[idx as usize];
            if entry.$field.as_bytes() == key_bytes {
                break Some(entry);
            }
            slot = (slot + 1) & mask;
        }
    }};
}

macro_rules! lookup_minecraft_path_entry {
    ($table_fn:ident, $slice:ident, $field:ident, $key:expr) => {{
        let path_bytes = default_minecraft_path($key)?;
        let table = $table_fn();
        let mask = table.len() - 1;
        let mut slot = (fnv1a_bytes(path_bytes) as usize) & mask;
        loop {
            let idx = table[slot];
            if idx == EMPTY_SLOT {
                break None;
            }
            let entry = &$slice[idx as usize];
            if strip_minecraft_prefix(entry.$field) == Some(path_bytes) {
                break Some(entry);
            }
            slot = (slot + 1) & mask;
        }
    }};
}

define_exact_entry_table!(block_type_table, 4096, VANILLA_BLOCK_TYPES, resource_id);
define_minecraft_path_entry_table!(item_type_table, 4096, VANILLA_ITEM_TYPES, resource_id);
define_minecraft_path_entry_table!(entity_type_table, 512, VANILLA_ENTITY_TYPES, resource_id);
define_minecraft_path_entry_table!(
    block_entity_type_table,
    128,
    VANILLA_BLOCK_ENTITY_TYPES,
    resource_id
);
define_minecraft_path_entry_table!(
    map_decoration_type_table,
    128,
    MAP_DECORATION_CATALOG,
    resource_id
);
define_exact_entry_table!(
    particle_type_table,
    256,
    VANILLA_PARTICLE_TYPES,
    resource_id
);
define_exact_entry_table!(
    data_component_type_table,
    256,
    VANILLA_DATA_COMPONENT_TYPES,
    resource_id
);
define_exact_entry_table!(
    attribute_type_table,
    128,
    VANILLA_ATTRIBUTE_TYPES,
    resource_id
);
define_exact_entry_table!(
    mob_effect_type_table,
    128,
    VANILLA_MOB_EFFECT_TYPES,
    resource_id
);
define_exact_entry_table!(stat_type_table, 32, VANILLA_STAT_TYPES, resource_id);
define_exact_entry_table!(custom_stat_table, 256, VANILLA_CUSTOM_STATS, resource_id);
define_exact_entry_table!(
    built_in_registry_type_table,
    256,
    VANILLA_BUILT_IN_REGISTRY_TYPES,
    resource_id
);

const _: () = assert!(VANILLA_SOUND_EVENT_RESOURCE_IDS.len() * 2 <= 4096);

#[inline]
fn sound_event_table() -> &'static [u16; 4096] {
    static TABLE: OnceLock<[u16; 4096]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [EMPTY_SLOT; 4096];
        let mask = 4096 - 1;
        for (idx, &resource_id) in VANILLA_SOUND_EVENT_RESOURCE_IDS.iter().enumerate() {
            let key = resource_id.as_bytes();
            let mut slot = (fnv1a_bytes(key) as usize) & mask;
            loop {
                let existing = table[slot];
                if existing == EMPTY_SLOT {
                    table[slot] = idx as u16;
                    break;
                }
                if VANILLA_SOUND_EVENT_RESOURCE_IDS[existing as usize].as_bytes() == key {
                    break;
                }
                slot = (slot + 1) & mask;
            }
        }
        table
    })
}

const _: () = assert!(VANILLA_STATIC_REGISTRY_ELEMENTS.len() * 2 <= 256);

#[inline]
fn static_registry_elements_table() -> &'static [u16; 256] {
    static TABLE: OnceLock<[u16; 256]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [EMPTY_SLOT; 256];
        let mask = 256 - 1;
        for (idx, &(registry_id, _)) in VANILLA_STATIC_REGISTRY_ELEMENTS.iter().enumerate() {
            let key = registry_id.as_bytes();
            let mut slot = (fnv1a_bytes(key) as usize) & mask;
            loop {
                let existing = table[slot];
                if existing == EMPTY_SLOT {
                    table[slot] = idx as u16;
                    break;
                }
                if VANILLA_STATIC_REGISTRY_ELEMENTS[existing as usize]
                    .0
                    .as_bytes()
                    == key
                {
                    break;
                }
                slot = (slot + 1) & mask;
            }
        }
        table
    })
}

#[inline]
pub(crate) fn block_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static BlockTypeCatalogEntry> {
    lookup_exact_entry!(
        block_type_table,
        VANILLA_BLOCK_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn beacon_beam_dye_color_id_for_block_name(resource_id: &str) -> Option<i32> {
    block_type_for_resource_id(resource_id)?.beacon_beam_dye_color_id
}

#[inline]
pub(crate) fn block_sound_group_by_resource_id_26_1(
    resource_id: &str,
) -> Option<&'static VanillaBlockSoundGroupFacts26_1> {
    let block = block_type_for_resource_id(resource_id)?;
    vanilla_block_sound_group_26_1(block.id)
}

#[inline]
pub(crate) fn item_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static ItemTypeCatalogEntry> {
    lookup_minecraft_path_entry!(
        item_type_table,
        VANILLA_ITEM_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn sound_event_registry_id(resource_id: &str) -> Option<i32> {
    let table = sound_event_table();
    let key_bytes = resource_id.as_bytes();
    let mask = table.len() - 1;
    let mut slot = (fnv1a_bytes(key_bytes) as usize) & mask;
    loop {
        let idx = table[slot];
        if idx == EMPTY_SLOT {
            return None;
        }
        if VANILLA_SOUND_EVENT_RESOURCE_IDS[idx as usize].as_bytes() == key_bytes {
            return Some(i32::from(idx));
        }
        slot = (slot + 1) & mask;
    }
}

#[inline]
pub(crate) fn entity_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static EntityTypeCatalogEntry> {
    lookup_minecraft_path_entry!(
        entity_type_table,
        VANILLA_ENTITY_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn entity_type_id_for_resource_id(resource_id: &str) -> Option<i32> {
    Some(entity_type_for_resource_id(resource_id)?.id)
}

#[inline]
pub(crate) fn block_entity_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static BlockEntityTypeCatalogEntry> {
    lookup_minecraft_path_entry!(
        block_entity_type_table,
        VANILLA_BLOCK_ENTITY_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn block_entity_type_id_for_resource_id(resource_id: &str) -> Option<i32> {
    Some(block_entity_type_for_resource_id(resource_id)?.id)
}

#[inline]
pub(crate) fn map_decoration_type_for_resource_id(
    resource_id: &str,
) -> Option<VanillaMapDecorationType> {
    lookup_minecraft_path_entry!(
        map_decoration_type_table,
        MAP_DECORATION_CATALOG,
        resource_id,
        resource_id
    )
    .copied()
}

#[inline]
pub(crate) fn particle_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static ParticleTypeCatalogEntry> {
    lookup_exact_entry!(
        particle_type_table,
        VANILLA_PARTICLE_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn data_component_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static DataComponentTypeCatalogEntry> {
    lookup_exact_entry!(
        data_component_type_table,
        VANILLA_DATA_COMPONENT_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn attribute_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static AttributeTypeCatalogEntry> {
    lookup_exact_entry!(
        attribute_type_table,
        VANILLA_ATTRIBUTE_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn mob_effect_type_for_resource_id(
    resource_id: &str,
) -> Option<&'static MobEffectTypeCatalogEntry> {
    lookup_exact_entry!(
        mob_effect_type_table,
        VANILLA_MOB_EFFECT_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn stat_type_by_resource_id(resource_id: &str) -> Option<&'static StatTypeCatalogEntry> {
    lookup_exact_entry!(
        stat_type_table,
        VANILLA_STAT_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn custom_stat_by_resource_id(
    resource_id: &str,
) -> Option<&'static CustomStatCatalogEntry> {
    lookup_exact_entry!(
        custom_stat_table,
        VANILLA_CUSTOM_STATS,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn built_in_registry_type(resource_id: &str) -> Option<&'static BuiltInRegistryType> {
    lookup_exact_entry!(
        built_in_registry_type_table,
        VANILLA_BUILT_IN_REGISTRY_TYPES,
        resource_id,
        resource_id
    )
}

#[inline]
pub(crate) fn static_registry_elements(registry: &str) -> Option<&'static [&'static str]> {
    let table = static_registry_elements_table();
    let key_bytes = registry.as_bytes();
    let mask = table.len() - 1;
    let mut slot = (fnv1a_bytes(key_bytes) as usize) & mask;
    loop {
        let idx = table[slot];
        if idx == EMPTY_SLOT {
            return None;
        }
        let (candidate, elements) = VANILLA_STATIC_REGISTRY_ELEMENTS[idx as usize];
        if candidate.as_bytes() == key_bytes {
            return Some(elements);
        }
        slot = (slot + 1) & mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_resource_id_tables_round_trip_every_entry_and_reject_bare_paths() {
        for entry in VANILLA_BLOCK_TYPES {
            assert_eq!(
                crate::block_types::vanilla_block_type_for_resource_id(entry.resource_id),
                Some(entry)
            );
            assert_eq!(
                crate::block_types::vanilla_beacon_beam_dye_color_id_for_block_name(
                    entry.resource_id
                ),
                entry.beacon_beam_dye_color_id
            );
            assert_eq!(
                crate::block_sound_facts_26_1::vanilla_block_sound_group_by_resource_id_26_1(
                    entry.resource_id
                )
                .map(|group| group.block_type_id),
                Some(entry.id)
            );
            let bare = entry.resource_id.strip_prefix("minecraft:").unwrap();
            assert_eq!(
                crate::block_types::vanilla_block_type_for_resource_id(bare),
                None
            );
        }
        assert_eq!(
            crate::block_types::vanilla_block_type_for_resource_id("minecraft:missing"),
            None
        );
        assert_eq!(
            crate::block_types::vanilla_block_type_for_resource_id(""),
            None
        );

        for (id, &resource_id) in VANILLA_SOUND_EVENT_RESOURCE_IDS.iter().enumerate() {
            assert_eq!(
                crate::sound_event_types::vanilla_sound_event_registry_id(resource_id),
                Some(id as i32)
            );
            let bare = resource_id.strip_prefix("minecraft:").unwrap();
            assert_eq!(
                crate::sound_event_types::vanilla_sound_event_registry_id(bare),
                None
            );
        }

        for entry in VANILLA_PARTICLE_TYPES {
            assert_eq!(
                crate::particle_types::vanilla_particle_type_for_resource_id(entry.resource_id),
                Some(entry)
            );
        }
        for entry in VANILLA_DATA_COMPONENT_TYPES {
            assert_eq!(
                crate::data_component_types::vanilla_data_component_type_for_resource_id(
                    entry.resource_id
                ),
                Some(entry)
            );
        }
        for entry in VANILLA_ATTRIBUTE_TYPES {
            assert_eq!(
                crate::attribute_types::vanilla_attribute_type_for_resource_id(entry.resource_id),
                Some(entry)
            );
        }
        for entry in VANILLA_MOB_EFFECT_TYPES {
            assert_eq!(
                crate::mob_effect_types::vanilla_mob_effect_type_for_resource_id(entry.resource_id),
                Some(entry)
            );
        }
        for entry in VANILLA_STAT_TYPES {
            assert_eq!(
                crate::stat_types::vanilla_stat_type_by_resource_id(entry.resource_id),
                Some(entry)
            );
        }
        for entry in VANILLA_CUSTOM_STATS {
            assert_eq!(
                crate::stat_types::vanilla_custom_stat_by_resource_id(entry.resource_id),
                Some(entry)
            );
        }
        for entry in &VANILLA_BUILT_IN_REGISTRY_TYPES {
            assert_eq!(
                crate::registry_data_types::built_in_registry_type(entry.resource_id),
                Some(entry)
            );
        }
        for &(registry, elements) in VANILLA_STATIC_REGISTRY_ELEMENTS {
            assert_eq!(
                crate::static_registry_elements::vanilla_static_registry_elements(registry),
                Some(elements)
            );
        }
        assert_eq!(
            crate::static_registry_elements::vanilla_static_registry_elements("minecraft:missing"),
            None
        );
    }

    #[test]
    fn default_namespace_tables_accept_bare_and_prefixed_paths_and_reject_foreign_namespaces() {
        for entry in VANILLA_ITEM_TYPES {
            let bare = entry.resource_id.strip_prefix("minecraft:").unwrap();
            assert_eq!(
                crate::item_types::vanilla_item_type_for_resource_id(entry.resource_id),
                Some(entry)
            );
            assert_eq!(
                crate::item_types::vanilla_item_type_for_resource_id(bare),
                Some(entry)
            );
        }
        assert_eq!(
            crate::item_types::vanilla_item_type_for_resource_id(""),
            None
        );
        assert_eq!(
            crate::item_types::vanilla_item_type_for_resource_id("minecraft:"),
            None
        );
        assert_eq!(
            crate::item_types::vanilla_item_type_for_resource_id("other:stone"),
            None
        );

        for entry in VANILLA_ENTITY_TYPES {
            let bare = entry.resource_id.strip_prefix("minecraft:").unwrap();
            assert_eq!(
                crate::entity_types::vanilla_entity_type_for_resource_id(entry.resource_id),
                Some(entry)
            );
            assert_eq!(
                crate::entity_types::vanilla_entity_type_for_resource_id(bare),
                Some(entry)
            );
            assert_eq!(
                crate::entity_types::vanilla_entity_type_id_for_resource_id(bare),
                Some(entry.id)
            );
        }
        assert_eq!(
            crate::entity_types::vanilla_entity_type_for_resource_id("other:pig"),
            None
        );

        for entry in VANILLA_BLOCK_ENTITY_TYPES {
            let bare = entry.resource_id.strip_prefix("minecraft:").unwrap();
            assert_eq!(
                crate::block_entity_types::vanilla_block_entity_type_for_resource_id(
                    entry.resource_id
                ),
                Some(entry)
            );
            assert_eq!(
                crate::block_entity_types::vanilla_block_entity_type_id_for_resource_id(bare),
                Some(entry.id)
            );
        }

        for entry in MAP_DECORATION_CATALOG {
            let bare = entry.resource_id.strip_prefix("minecraft:").unwrap();
            assert_eq!(
                crate::map_decorations::vanilla_map_decoration_type_for_resource_id(
                    entry.resource_id
                ),
                Some(*entry)
            );
            assert_eq!(
                crate::map_decorations::vanilla_map_decoration_type_for_resource_id(bare),
                Some(*entry)
            );
        }
    }
}
