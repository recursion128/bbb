//! Server-side plugin registry binding frozen content specs to mob behaviors.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use bbb_protocol::codec::Result as ProtocolResult;
use bbb_protocol::content_registry::{
    ContentRegistry, ContentRegistryBuilder, ModBlockRegistration, ModEntityRegistration,
    ModEntityTypeSpec, ModItemRegistration, ModParticleRegistration,
};

pub use crate::entity::plugin_mob::{
    DefaultPluginMobBehavior, MobBehaviorSpec, MobGoalSpec, PluginMobBehavior, PluginMobState,
};

#[derive(Debug, Clone)]
pub struct RegisteredPluginMob {
    pub spec: ModEntityTypeSpec,
    pub behavior_spec: MobBehaviorSpec,
    pub behavior: Arc<dyn PluginMobBehavior>,
}

/// Immutable per-world plugin registry paired with the frozen session
/// [`ContentRegistry`].
#[derive(Debug)]
pub struct ServerPluginRegistry {
    content_registry: Arc<ContentRegistry>,
    mobs_by_offset: Vec<RegisteredPluginMob>,
}

impl Default for ServerPluginRegistry {
    fn default() -> Self {
        Self {
            content_registry: Arc::clone(ContentRegistry::vanilla()),
            mobs_by_offset: Vec::new(),
        }
    }
}

impl ServerPluginRegistry {
    pub fn vanilla() -> &'static Arc<Self> {
        static VANILLA: OnceLock<Arc<ServerPluginRegistry>> = OnceLock::new();
        VANILLA.get_or_init(|| Arc::new(Self::default()))
    }

    pub fn is_vanilla_only(&self) -> bool {
        self.content_registry.is_vanilla_only() && self.mobs_by_offset.is_empty()
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    pub fn mob_entry(&self, entity_type_id: i32) -> Option<&RegisteredPluginMob> {
        let offset = usize::try_from(entity_type_id - ContentRegistry::entity_base_id()).ok()?;
        self.mobs_by_offset.get(offset)
    }

    pub fn mob_entry_by_resource_id(&self, resource_id: &str) -> Option<&RegisteredPluginMob> {
        let spec = self.content_registry.entity_by_resource_id(resource_id)?;
        self.mob_entry(spec.id)
    }
}

#[derive(Debug, Default)]
pub struct ServerPluginRegistryBuilder {
    content: ContentRegistryBuilder,
    pending_mobs: Vec<(String, MobBehaviorSpec, Arc<dyn PluginMobBehavior>)>,
}

impl ServerPluginRegistryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_mob(
        &mut self,
        registration: ModEntityRegistration,
        behavior_spec: MobBehaviorSpec,
        behavior: Arc<dyn PluginMobBehavior>,
    ) -> &mut Self {
        let normalized_key =
            bbb_protocol::packets::normalize_resource_location(registration.resource_id.clone())
                .unwrap_or_else(|_| registration.resource_id.clone());
        self.content.register_entity(registration);
        self.pending_mobs
            .push((normalized_key, behavior_spec, behavior));
        self
    }

    pub fn register_item(&mut self, spec: ModItemRegistration) -> &mut Self {
        self.content.register_item(spec);
        self
    }

    pub fn register_block(&mut self, spec: ModBlockRegistration) -> &mut Self {
        self.content.register_block(spec);
        self
    }

    pub fn register_sound_event(&mut self, resource_id: impl Into<String>) -> &mut Self {
        self.content.register_sound_event(resource_id);
        self
    }

    pub fn register_particle(&mut self, spec: ModParticleRegistration) -> &mut Self {
        self.content.register_particle(spec);
        self
    }

    pub fn freeze(self) -> ProtocolResult<ServerPluginRegistry> {
        let content_registry = Arc::new(self.content.freeze()?);
        let mut behavior_map: HashMap<String, (MobBehaviorSpec, Arc<dyn PluginMobBehavior>)> = self
            .pending_mobs
            .into_iter()
            .map(|(id, spec, behavior)| (id, (spec, behavior)))
            .collect();
        let mut mobs_by_offset = Vec::with_capacity(content_registry.entities().len());
        for entity_spec in content_registry.entities() {
            let (behavior_spec, behavior) = behavior_map
                .remove(&entity_spec.resource_id)
                .unwrap_or_else(|| {
                    (
                        MobBehaviorSpec::default(),
                        crate::entity::plugin_mob::shared_default_behavior(),
                    )
                });
            mobs_by_offset.push(RegisteredPluginMob {
                spec: entity_spec.clone(),
                behavior_spec,
                behavior,
            });
        }
        Ok(ServerPluginRegistry {
            content_registry,
            mobs_by_offset,
        })
    }
}
