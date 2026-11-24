//! `SmallFireball` leaf state and ordered hit effects.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID;
use bbb_protocol::packets::{BlockPos, Direction, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::living::SynchedValue;
use crate::entity::projectile_action::{
    PendingProjectileAction, PendingProjectileHurt, PendingProjectileIgniteHurt,
    ProjectileLandOrigin,
};
use crate::entity::projectile_hit::HitResult;
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::throwable_projectile::{pending_owner, pending_target};
use crate::entity::{Entity, EntityData, EntityStore};
use crate::level_event::PendingHurtEntity;
use bbb_server_types::fluid::block_state_is_air;

#[derive(Debug, Clone, PartialEq)]
pub struct SmallFireballData {
    projectile: HurtingProjectileState,
    item: RuntimeItemStack,
}

impl SmallFireballData {
    pub(crate) fn replace_slot_item(&mut self, item: RuntimeItemStack) {
        self.item = item;
    }

    pub const fn new(projectile: HurtingProjectileState, item: RuntimeItemStack) -> Self {
        Self { projectile, item }
    }

    pub const fn projectile(&self) -> &HurtingProjectileState {
        &self.projectile
    }

    pub fn projectile_mut(&mut self) -> &mut HurtingProjectileState {
        &mut self.projectile
    }

    pub const fn item(&self) -> &RuntimeItemStack {
        &self.item
    }
}

pub fn synched_values(entity: &Entity, data: &SmallFireballData) -> Vec<SynchedValue> {
    super::fireball::synched_values_for_item(entity, data.item())
}

pub fn spawn_small_fireball(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    owner: Option<Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
        position,
        EntityData::SmallFireball(SmallFireballData::new(
            HurtingProjectileState::new(owner, HurtingProjectileState::INITIAL_ACCELERATION_POWER),
            super::fireball::default_item(),
        )),
        |entity| shot.install(entity),
    )
}

pub(super) fn on_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    owner: Option<Uuid>,
    hit: HitResult,
) -> bool {
    match hit {
        HitResult::Entity {
            target, location, ..
        } => {
            let causing = pending_owner(owner, store, level);
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::IgniteThenHurt(PendingProjectileIgniteHurt {
                    hurt: PendingProjectileHurt {
                        target: pending_target(target),
                        damage_type: if causing.is_some() {
                            "minecraft:fireball"
                        } else {
                            "minecraft:unattributed_fireball"
                        },
                        amount: 5.0,
                        causing,
                        direct: Some(PendingHurtEntity::Stored {
                            entity_id: entity.id(),
                        }),
                        direct_resource_id: "minecraft:small_fireball",
                        registry_tags: level.capture_registry_tags(),
                    },
                    fire_ticks: 100,
                    restore_fire_on_refusal: true,
                }),
            ));
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Land {
                    origin: ProjectileLandOrigin::Entity(location),
                    source_entity_id: entity.id(),
                },
            ));
        }
        HitResult::Block { pos, direction, .. } => {
            let fire_pos = relative(pos, direction);
            if may_place_fire(owner, store, level)
                && block_state_is_air(level.block_state(fire_pos))
            {
                store.queue_command(crate::entity::EntityCommand::Projectile(
                    PendingProjectileAction::PlaceFire { pos: fire_pos },
                ));
            }
            store.queue_command(crate::entity::EntityCommand::Projectile(
                PendingProjectileAction::Land {
                    origin: ProjectileLandOrigin::Block(pos),
                    source_entity_id: entity.id(),
                },
            ));
        }
        HitResult::Miss => unreachable!("the shared tick returns only non-miss hits"),
    }
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Discard {
            entity_id: entity.id(),
        },
    ));
    true
}

fn may_place_fire(owner: Option<Uuid>, store: &EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    let owner_is_mob = owner
        .and_then(|uuid| store.entity_id_for_uuid(uuid))
        .and_then(|id| store.get(id))
        .is_some_and(|entity| matches!(entity.data(), EntityData::GeneralLiving(_)));
    !owner_is_mob || level.mob_griefing()
}

const fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        Direction::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        Direction::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}
