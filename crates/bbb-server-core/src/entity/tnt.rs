//! `PrimedTnt` (`net/minecraft/world/entity/item/PrimedTnt.java`): the block a
//! lit fuse becomes, which falls, counts down and queues an explosion.
//!
//! Damage immunity and the portal path are transcribed; the fluid interaction
//! `PrimedTnt.tick` runs (`:114`) has no owner in this crate (see
//! [`crate`]'s module documentation) and is the one omission.

use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID, VANILLA_PRIMED_TNT_FUSE_DATA_ID,
};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TNT_ID;
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use uuid::Uuid;

use crate::entity::living::SynchedValue;
use crate::entity::store::EntityStore;
use crate::entity::{apply_gravity, handle_portal, Entity, EntityData, MoverType};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use bbb_server_types::geometry::Vec3Ops;

/// `PrimedTnt.getDefaultGravity()` (`PrimedTnt.java:91-93`).
pub const DEFAULT_GRAVITY: f64 = 0.04;

/// `PrimedTnt.DEFAULT_FUSE_TIME` (`PrimedTnt.java:34`).
pub const DEFAULT_FUSE_TIME: i32 = 80;

/// `PrimedTnt.DEFAULT_EXPLOSION_POWER` (`PrimedTnt.java:35`).
pub const DEFAULT_EXPLOSION_POWER: f32 = 4.0;

/// `Mth.clamp` bound `readAdditionalSaveData` holds `explosion_power` to
/// (`PrimedTnt.java:153`).
pub const MAX_EXPLOSION_POWER: f32 = 128.0;

/// `(float)(Math.PI * 2)` widened to the double `PrimedTnt`'s constructor
/// multiplies `nextDouble` by (`PrimedTnt.java:65`, `javap` `ldc2_w`).
pub const TWO_PI_F32_AS_F64: f64 = 6.283_185_482_025_146_5;

/// The `PrimedTnt` fields this slice carries.
#[derive(Debug, Clone, PartialEq)]
pub struct TntData {
    fuse: i32,
    block_state_id: i32,
    owner: Option<Uuid>,
    explosion_power: f32,
    used_portal: bool,
}

pub fn default_block_state_id() -> i32 {
    vanilla_block_default_state_id(VANILLA_BLOCK_TNT_ID)
        .expect("the built-in registry carries minecraft:tnt")
}

impl TntData {
    pub fn new() -> Self {
        Self {
            fuse: DEFAULT_FUSE_TIME,
            block_state_id: default_block_state_id(),
            owner: None,
            explosion_power: DEFAULT_EXPLOSION_POWER,
            used_portal: false,
        }
    }

    pub fn with_saved_fields(
        fuse: i32,
        block_state_id: i32,
        owner: Option<Uuid>,
        explosion_power: f32,
    ) -> Self {
        Self {
            fuse,
            block_state_id,
            owner,
            explosion_power,
            used_portal: false,
        }
    }

    pub const fn fuse(&self) -> i32 {
        self.fuse
    }

    pub const fn block_state_id(&self) -> i32 {
        self.block_state_id
    }

    pub const fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub const fn explosion_power(&self) -> f32 {
        self.explosion_power
    }

    pub const fn used_portal(&self) -> bool {
        self.used_portal
    }

    pub fn set_fuse(&mut self, fuse: i32) {
        self.fuse = fuse;
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.used_portal = previous.used_portal;
    }
}

pub fn mark_used_portal_after_teleport(entity: &mut Entity) {
    if let EntityData::Tnt(tnt) = &mut entity.data {
        tnt.used_portal = true;
    }
}

impl Default for TntData {
    fn default() -> Self {
        Self::new()
    }
}

///
/// The constructor's one draw is `level.getRandom().nextDouble()` (`:65`), the
/// *level* random rather than the entity's own, so `random` is the world random
/// the block-use and chain-reaction callers already hold; the chain fuse
/// (`TntBlock.wasExploded`, `:78`) is the very next draw off that same stream.
/// The UUID still comes from the store, which mints every entity's id.
pub fn spawn_tnt(
    store: &mut EntityStore,
    random: &GameRandom,
    position: Vec3d,
    owner: Option<Uuid>,
) -> Option<i32> {
    let uuid = store.next_uuid();
    // `:65-66`: `rot = nextDouble * 2PI`, motion `(-sin rot * 0.02, 0.2, -cos rot * 0.02)`.
    let rot = random.next_double() * TWO_PI_F32_AS_F64;
    let delta = Vec3d {
        x: -rot.sin() * 0.02,
        y: f64::from(0.2_f32),
        z: -rot.cos() * 0.02,
    };
    let data = TntData {
        owner,
        ..TntData::new()
    };
    let id = store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_TNT_ID,
        position,
        EntityData::Tnt(data),
        |entity| {
            entity.set_delta_movement(delta);
            entity.set_needs_sync(true);
        },
    )?;
    Some(id)
}

pub fn spawn_chain_reaction_tnt(
    store: &mut EntityStore,
    random: &GameRandom,
    position: Vec3d,
    owner: Option<Uuid>,
) -> Option<i32> {
    let id = spawn_tnt(store, random, position, owner)?;
    let fuse = random.next_int(DEFAULT_FUSE_TIME / 4) + DEFAULT_FUSE_TIME / 8;
    if let Some(entity) = store.get_mut(id) {
        if let EntityData::Tnt(tnt) = &mut entity.data {
            tnt.set_fuse(fuse);
        }
        entity.mark_synched_dirty(VANILLA_PRIMED_TNT_FUSE_DATA_ID);
    }
    Some(id)
}

fn data(entity: &Entity) -> &TntData {
    match entity.data() {
        EntityData::Tnt(tnt) => tnt,
        other => unreachable!("tnt tick dispatched a {other:?}"),
    }
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if !handle_portal(entity, level) {
        return false;
    }
    apply_gravity(entity);
    let delta = entity.delta_movement();
    if crate::entity::movement::move_entity_with_ground_contact(
        entity,
        store,
        level,
        MoverType::Self_,
        delta,
    )
    .is_err()
    {
        return false;
    }
    // `:100`.
    if !crate::entity::apply_nonliving_inside_block_effects(entity, level, Some(&mut *store)) {
        return false;
    }
    // `:101`.
    entity.set_delta_movement(entity.delta_movement().scale(0.98));
    // `:102-104`.
    if entity.on_ground() {
        entity.set_delta_movement(entity.delta_movement().multiply(0.7, -0.5, 0.7));
    }
    // `:106-107`.
    let fuse = data(entity).fuse() - 1;
    if let EntityData::Tnt(tnt) = &mut entity.data {
        tnt.set_fuse(fuse);
    }
    entity.mark_synched_dirty(VANILLA_PRIMED_TNT_FUSE_DATA_ID);
    if fuse <= 0 {
        // `:108-112`: discard, then explode server-side.
        entity.discard();
        queue_explosion(entity, store, level);
    } else if crate::entity::environment::update_fluid_interaction_in_level(entity, store, level)
        .is_none()
    {
        return false;
    }
    // `PrimedTnt.tick` overrides `Entity.tick` outright and never calls
    // `super.tick()`, so `Entity.baseTick` -- and its `checkBelowWorld` -- do
    // not run for a primed TNT.
    true
}

// PrimedTnt.java:121-136.
fn queue_explosion(entity: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    if !level.tnt_explodes() {
        return;
    }
    let tnt = data(entity);
    let position = entity.position();
    let direct = crate::entity::damage::explosion_source_entity(entity);
    let indirect = tnt.owner().and_then(|owner| {
        crate::entity::damage::explosion_owner_source_entity(owner, store, level)
    });
    let request = PendingExplosion {
        source: Some(ExplosionSource {
            direct,
            indirect: indirect.clone(),
            causing: indirect,
            in_water: entity.was_touching_water(),
            wind_charge: false,
        }),
        center: Vec3d {
            x: position.x,
            y: position.y + (entity.bounding_box().max_y - entity.bounding_box().min_y) * 0.0625,
            z: position.z,
        },
        radius: tnt.explosion_power(),
        fire: false,
        interaction: ExplosionInteraction::Tnt,
        damage_source: None,
        damage_profile: if tnt.used_portal() {
            ExplosionDamageProfile::PrimedTntUsedPortal
        } else {
            ExplosionDamageProfile::Default
        },
        presentation: ExplosionPresentation::Default,
    };
    store.queue_command(crate::entity::EntityCommand::Explosion(request));
}

pub fn synched_values(entity: &Entity, data: &TntData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_PRIMED_TNT_FUSE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(data.fuse()),
        },
        // `defineSynchedData` defaults the fuse to 80 (`:76`).
        is_default: data.fuse() == DEFAULT_FUSE_TIME,
    });
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_PRIMED_TNT_BLOCK_STATE_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BLOCK_STATE_ID,
            value: EntityDataValueKind::BlockState(data.block_state_id()),
        },
        // `defineSynchedData` defaults the block state to TNT's own (`:77`).
        is_default: data.block_state_id() == default_block_state_id(),
    });
    rows
}
