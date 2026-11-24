use std::sync::Arc;

use crate::entity::save::EntityNbtReadContext;
use crate::registry_tags::RegistryTagSnapshot;

use crate::world::ServerWorld;

#[derive(Clone)]
pub(in crate::world) struct EntityNbtReadSnapshot {
    tags: Arc<RegistryTagSnapshot>,
    levels: Arc<crate::world::level_keys::LevelNames>,
}

impl EntityNbtReadSnapshot {
    pub(in crate::world) fn level_names(&self) -> &crate::world::level_keys::LevelNames {
        &self.levels
    }

    pub(in crate::world) fn level_key(
        &self,
        id: crate::world::LevelId,
    ) -> &bbb_server_types::save::Java26LevelKey {
        self.levels.key(id)
    }

    pub(in crate::world) fn context(&self) -> EntityNbtReadContext<'_> {
        EntityNbtReadContext::new(&self.tags)
    }

    pub(in crate::world) fn content_registry(
        &self,
    ) -> &bbb_protocol::content_registry::ContentRegistry {
        self.tags.content_registry()
    }
}

impl ServerWorld {
    pub(in crate::world) fn entity_nbt_read_snapshot(&self) -> EntityNbtReadSnapshot {
        EntityNbtReadSnapshot {
            tags: self.registries.registry_tags_shared(),
            levels: Arc::clone(&self.levels.names),
        }
    }
}
