//! Item holder publications over the packaged vanilla datapack for tests.

use std::sync::{Arc, OnceLock};

use bbb_protocol::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

use crate::entity::ItemPublication;

pub(crate) fn builtin_startup() -> &'static DatapackStartup {
    static STARTUP: OnceLock<DatapackStartup> = OnceLock::new();
    STARTUP.get_or_init(|| {
        DatapackStartup::load(&DatapackStartupOptions {
            datapacks_dir: None,
            saved_data_packs: None,
            safe_mode: false,
            limits: DatapackLoadLimits::default(),
        })
        .expect("the embedded vanilla datapack loads")
    })
}

/// The packaged vanilla datapack's registries as an item holder publication.
/// A tag the pack does not declare reads as an empty tag, so fixtures may name
/// the custom tags their level snapshot supplies.
pub(crate) struct BuiltinItems;

impl PersistentRegistryResolver for BuiltinItems {
    fn registry_available(&self, registry_id: &str) -> bool {
        PersistentRegistryResolver::registry_available(
            &FrozenDatapackRegistryLookup::from_startup(builtin_startup()),
            registry_id,
        )
    }

    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        PersistentRegistryResolver::resource_membership(
            &FrozenDatapackRegistryLookup::from_startup(builtin_startup()),
            registry_id,
            resource_id,
        )
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        match PersistentRegistryResolver::tag_members(
            &FrozenDatapackRegistryLookup::from_startup(builtin_startup()),
            registry_id,
            tag_id,
        ) {
            PersistentRegistryTagMembers::Absent => {
                PersistentRegistryTagMembers::Present(Vec::new())
            }
            members => members,
        }
    }
}

impl PersistentItemStackHolderResolver for BuiltinItems {
    fn cat_sound_variant(
        &self,
        holder_id: i32,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CatSoundVariantElement> {
        builtin_sound_registries().cat_sound_variant(holder_id)
    }

    fn cow_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::CowSoundVariantElement> {
        builtin_sound_registries().cow_sound_variant(resource_id)
    }

    fn chicken_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::ChickenSoundVariantElement>
    {
        builtin_sound_registries().chicken_sound_variant(resource_id)
    }

    fn pig_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::PigSoundVariantElement> {
        builtin_sound_registries().pig_sound_variant(resource_id)
    }

    fn wolf_sound_variant(
        &self,
        resource_id: &str,
    ) -> Option<&bbb_registry_codec::registry_codec::sound_variants::WolfSoundVariantElement> {
        builtin_sound_registries().wolf_sound_variant(resource_id)
    }

    fn wolf_sound_variant_count(&self) -> Option<usize> {
        builtin_sound_registries().registry_len("minecraft:wolf_sound_variant")
    }

    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        PersistentItemStackHolderResolver::holder_id(
            &FrozenDatapackRegistryLookup::from_startup(builtin_startup()),
            registry_id,
            resource_id,
        )
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        PersistentItemStackHolderResolver::holder_resource_id(
            &FrozenDatapackRegistryLookup::from_startup(builtin_startup()),
            registry_id,
            holder_id,
        )
    }
}

fn builtin_sound_registries() -> &'static crate::registry_sync::SynchronizedRegistries {
    static REGISTRIES: OnceLock<crate::registry_sync::SynchronizedRegistries> = OnceLock::new();
    REGISTRIES.get_or_init(|| crate::registry_sync::SynchronizedRegistries::load(None).unwrap())
}

impl ItemPublication {
    pub(crate) fn builtin() -> &'static Self {
        static ITEMS: OnceLock<ItemPublication> = OnceLock::new();
        ITEMS.get_or_init(|| ItemPublication::new(Arc::new(BuiltinItems)))
    }
}
