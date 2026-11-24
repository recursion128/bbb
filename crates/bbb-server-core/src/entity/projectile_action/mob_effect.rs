use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_SILVERFISH_ID, VANILLA_ENTITY_TYPE_SLIME_ID};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_types::geometry::Aabb;

use crate::entity::{EntityData, EntityStore, GeneralLivingData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingRemovalMobEffect {
    Oozing,
    Weaving,
    WindCharged,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemovedLivingSnapshot {
    pub entity_id: i32,
    pub entity_resource_id: &'static str,
    pub uuid: uuid::Uuid,
    pub is_mob: bool,
    pub position: Vec3d,
    pub block_position: BlockPos,
    pub bounding_box: Aabb,
    pub was_touching_water: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingRemovalMobEffects {
    pub source: RemovedLivingSnapshot,
    pub effects: Vec<PendingRemovalMobEffect>,
    pub removed_entity: Box<crate::entity::Entity>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingMobEffectAction {
    Removed(PendingRemovalMobEffects),
    Infested {
        position: Vec3d,
        velocities: Vec<Vec3d>,
    },
}

pub fn spawn_oozing_slime(store: &mut EntityStore, position: Vec3d, y_rot: f32) -> Option<i32> {
    spawn_effect_mob(
        store,
        VANILLA_ENTITY_TYPE_SLIME_ID,
        position,
        y_rot,
        |entity| {
            crate::entity::slime::initialize_spawn(entity);
            let _ = crate::entity::slime::set_size(entity, 2, true);
        },
    )
}

pub fn spawn_infested_silverfish(
    store: &mut EntityStore,
    position: Vec3d,
    y_rot: f32,
    velocity: Vec3d,
) -> Option<i32> {
    spawn_effect_mob(
        store,
        VANILLA_ENTITY_TYPE_SILVERFISH_ID,
        position,
        y_rot,
        |entity| entity.set_delta_movement(velocity),
    )
}

fn spawn_effect_mob(
    store: &mut EntityStore,
    entity_type_id: i32,
    position: Vec3d,
    y_rot: f32,
    initialize: impl FnOnce(&mut crate::entity::Entity),
) -> Option<i32> {
    let uuid = store.next_uuid();
    let data = GeneralLivingData::new(entity_type_id)?;
    store.restore_initialized(
        uuid,
        entity_type_id,
        position,
        EntityData::GeneralLiving(data),
        |entity| {
            entity.set_rot(y_rot, 0.0);
            entity
                .living_mut()
                .expect("effect child is general living")
                .y_head_rot = y_rot;
            initialize(entity);
        },
    )
}
