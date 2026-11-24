use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::{finalize_spawn, Group, SpawnContext, SpawnLevel, SpawnReason};
use crate::entity::save::create_fresh_entity;
use crate::entity::{Entity, EntityIdCounter, EntityStore};

/// A level with one fixed set of facts and no other entities.
pub(crate) struct TestLevel {
    pub(crate) store: EntityStore,
    pub(crate) ctx: SpawnContext<'static>,
}

impl TestLevel {
    pub(crate) fn new() -> Self {
        let mut store = EntityStore::new(0x46_49_4e_41_4c, EntityIdCounter::new());
        store.set_item_publication(crate::entity::ItemPublication::builtin().clone());
        Self {
            store,
            ctx: SpawnContext {
                game_time: 100,
                difficulty: Difficulty::Normal,
                special_multiplier: 0.0,
                dimension: "minecraft:overworld".into(),
                biome: Some("minecraft:plains".into()),
                biome_tags: Vec::new(),
                variants: Vec::new(),
                halloween: false,
            },
        }
    }

    pub(crate) fn variant(
        mut self,
        registry: &'static str,
        resource_id: &'static str,
        holder_id: i32,
    ) -> Self {
        self.ctx.variants.push((registry, resource_id, holder_id));
        self
    }

    pub(crate) fn create(&mut self, resource_id: &str) -> Box<Entity> {
        create_fresh_entity(
            &mut self.store,
            resource_id,
            Vec3d {
                x: 2.5,
                y: 70.0,
                z: -3.5,
            },
        )
        .expect("a built-in entity type")
    }

    /// Finalizes and publishes a fresh `resource_id`, answering its id and
    /// the group data the next member would inherit.
    pub(crate) fn spawn(
        &mut self,
        resource_id: &str,
        reason: SpawnReason,
        group: Group,
        seed: i64,
    ) -> (i32, Group) {
        let mut entity = self.create(resource_id);
        let group = finalize_spawn(
            self,
            &mut entity,
            reason,
            group,
            &GameRandom::with_seed(seed),
        )
        .expect("the test level answers every fact");
        (self.store.add_fresh(entity), group)
    }

    pub(crate) fn get(&self, id: i32) -> &Entity {
        self.store.get(id).expect("published entity")
    }
}

impl SpawnLevel<'static> for TestLevel {
    fn entities(&mut self) -> &mut EntityStore {
        &mut self.store
    }

    fn spawn_context(&self, _: &str, _: BlockPos) -> Option<SpawnContext<'static>> {
        Some(self.ctx.clone())
    }

    fn enchant(
        &self,
        _: &str,
        _: &RuntimeItemStack,
        _: f32,
        _: &GameRandom,
    ) -> Option<RuntimeItemStack> {
        None
    }

    fn first_unridden(&self, _: &str, _: Aabb) -> Option<i32> {
        None
    }

    fn no_collision(&self, _: Aabb) -> bool {
        true
    }
}
