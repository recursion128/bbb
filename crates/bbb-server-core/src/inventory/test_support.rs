use bbb_protocol::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupOptions};

pub(crate) struct TestItemResolver(DatapackStartup);

pub(crate) fn test_item_resolver() -> &'static TestItemResolver {
    static RESOLVER: std::sync::OnceLock<TestItemResolver> = std::sync::OnceLock::new();
    RESOLVER.get_or_init(|| {
        TestItemResolver(
            DatapackStartup::load(&DatapackStartupOptions {
                datapacks_dir: None,
                saved_data_packs: None,
                safe_mode: false,
                limits: Default::default(),
            })
            .expect("builtin test publication"),
        )
    })
}

impl PersistentRegistryResolver for TestItemResolver {
    fn registry_available(&self, registry: &str) -> bool {
        PersistentRegistryResolver::registry_available(
            &FrozenDatapackRegistryLookup::from_startup(&self.0),
            registry,
        )
    }
    fn resource_membership(&self, registry: &str, id: &str) -> PersistentRegistryMembership {
        FrozenDatapackRegistryLookup::from_startup(&self.0).resource_membership(registry, id)
    }
    fn tag_members(&self, registry: &str, id: &str) -> PersistentRegistryTagMembers {
        FrozenDatapackRegistryLookup::from_startup(&self.0).tag_members(registry, id)
    }
}

impl PersistentItemStackHolderResolver for TestItemResolver {
    fn jukebox_song(
        &self,
        holder_id: i32,
    ) -> Option<bbb_registry_codec::registry_codec::JukeboxSongRegistryValue> {
        let element = self
            .0
            .registries
            .registry("minecraft:jukebox_song")?
            .elements
            .get(usize::try_from(holder_id).ok()?)?;
        bbb_registry_codec::registry_codec::jukebox_song_registry_value(
            &element.codec_value.as_ref()?.network_nbt,
        )
    }

    fn holder_id(&self, registry: &str, id: &str) -> Option<i32> {
        FrozenDatapackRegistryLookup::from_startup(&self.0).holder_id(registry, id)
    }
    fn holder_resource_id(&self, registry: &str, id: i32) -> Option<String> {
        FrozenDatapackRegistryLookup::from_startup(&self.0).holder_resource_id(registry, id)
    }
}
