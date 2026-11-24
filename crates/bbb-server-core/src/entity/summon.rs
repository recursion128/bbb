//! `/summon` over the runtimes this crate carries.

use bbb_protocol::entity_types::{
    vanilla_entity_type, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID,
};
use bbb_protocol::packets::Vec3d;

use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityData};

/// What `SummonCommand.createEntity` built, before
/// `tryAddFreshEntityWithPassengers` adds it (`SummonCommand.java:83-102`).
pub enum SummonedEntity {
    /// The command's `finalize` arm is `entity instanceof Mob`, so a type that
    /// is not a mob publishes with nothing between the load and the add.
    Published(i32),
    /// A mob with its `Mob.finalizeSpawn` (`:94-96`) still to run against the
    /// level's own `RandomSource` and
    /// `getCurrentDifficultyAt(entity.blockPosition())`.
    NeedsFinalize(Box<Entity>),
}

impl EntityStore {
    /// The entity `createEntity` builds (`SummonCommand.java:83-92`).
    pub fn spawn_summoned(
        &mut self,
        entity_type_id: i32,
        position: Vec3d,
    ) -> Option<SummonedEntity> {
        if entity_type_id == VANILLA_ENTITY_TYPE_ARMOR_STAND_ID {
            return crate::entity::armor_stand::spawn_summoned(self, position)
                .map(SummonedEntity::Published);
        }
        if entity_type_id == VANILLA_ENTITY_TYPE_LIGHTNING_BOLT_ID {
            return crate::entity::lightning_bolt::spawn_lightning_bolt(
                self, position, false, None,
            )
            .map(SummonedEntity::Published);
        }
        let resource_id = vanilla_entity_type(entity_type_id)
            .map(|entry| entry.resource_id)
            .or_else(|| {
                self.plugins()
                    .content_registry()
                    .entity_by_id(entity_type_id)
                    .map(|spec| spec.resource_id_static())
            })?;
        let entity = crate::entity::save::create_fresh_entity(self, resource_id, position)?;
        Some(match entity.data() {
            EntityData::GeneralLiving(data) if data.is_mob() => {
                SummonedEntity::NeedsFinalize(entity)
            }
            _ => SummonedEntity::Published(self.add_fresh(entity)),
        })
    }
}
