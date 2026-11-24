//! `DragonFireball` runtime and its breath-cloud hit effect.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID;
use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::entity::area_effect_cloud::{spawn_area_effect_cloud, AreaEffectCloudData};
use crate::entity::hurting_projectile::HurtingProjectileState;
use crate::entity::projectile_action::{PendingProjectileAction, ProjectileLandOrigin};
use crate::entity::projectile_hit::{HitResult, ProjectileTarget};
use crate::entity::projectile_shoot::ProjectileShot;
use crate::entity::{Entity, EntityData, EntityStore};
use bbb_server_types::geometry::Vec3Ops;

#[derive(Debug, Clone, PartialEq)]
pub struct DragonFireballData {
    projectile: HurtingProjectileState,
}

impl DragonFireballData {
    pub const fn new(projectile: HurtingProjectileState) -> Self {
        Self { projectile }
    }

    pub const fn projectile(&self) -> &HurtingProjectileState {
        &self.projectile
    }

    pub fn projectile_mut(&mut self) -> &mut HurtingProjectileState {
        &mut self.projectile
    }
}

pub fn spawn_dragon_fireball(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    owner: Uuid,
) -> Option<i32> {
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_DRAGON_FIREBALL_ID,
        position,
        EntityData::DragonFireball(DragonFireballData::new(HurtingProjectileState::new(
            Some(owner),
            HurtingProjectileState::INITIAL_ACCELERATION_POWER,
        ))),
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
    let origin = match hit {
        HitResult::Block { pos, .. } => ProjectileLandOrigin::Block(pos),
        HitResult::Entity { location, .. } => ProjectileLandOrigin::Entity(location),
        HitResult::Miss => unreachable!("the shared tick returns only non-miss hits"),
    };
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Land {
            origin,
            source_entity_id: entity.id(),
        },
    ));
    if matches!(hit, HitResult::Entity { target, .. } if owned_target(target, owner, store)) {
        return true;
    }

    let cloud_position = nearby_living_position(entity, store, level).unwrap_or(entity.position());
    let cloud = AreaEffectCloudData::new()
        .with_radius(3.0)
        .with_duration(600)
        .with_radius_per_tick((7.0 - 3.0) / 600.0)
        .with_potion_duration_scale(0.25)
        .with_owner(living_owner(owner, store, level))
        .with_custom_particle(Some(dragon_breath_particle()))
        .with_potion_contents(Some(instant_damage_two()));
    if spawn_area_effect_cloud(store, cloud_position, cloud).is_none() {
        return true;
    }
    // `DragonFireball.java:57`.
    let data = if entity.is_silent() { -1 } else { 1 };
    level.level_event(2006, entity.block_position(), data);
    store.queue_command(crate::entity::EntityCommand::Projectile(
        PendingProjectileAction::Discard {
            entity_id: entity.id(),
        },
    ));
    true
}

fn owned_target(target: ProjectileTarget, owner: Option<Uuid>, store: &EntityStore) -> bool {
    let Some(owner) = owner else {
        return false;
    };
    match target {
        ProjectileTarget::Stored { entity_id } => store
            .get(entity_id)
            .is_some_and(|entity| entity.uuid() == owner),
        ProjectileTarget::Player { uuid, .. } => uuid == owner,
    }
}

fn living_owner(
    owner: Option<Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<Uuid> {
    let owner = owner?;
    if level.player(owner).is_some()
        || store
            .entity_id_for_uuid(owner)
            .and_then(|id| store.get(id))
            .is_some_and(|entity| entity.living().is_some())
    {
        Some(owner)
    } else {
        None
    }
}

fn nearby_living_position(
    source: &Entity,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<Vec3d> {
    let area = source.bounding_box().inflate(4.0, 2.0, 4.0);
    for id in store.ids_in_box_sections(area) {
        let Some(entity) = store.get(id) else {
            continue;
        };
        if entity.living().is_some() && source.position().distance_to_sqr(entity.position()) < 16.0
        {
            return Some(entity.position());
        }
    }
    level
        .players_in_box(area)
        .into_iter()
        .find(|player| source.position().distance_to_sqr(player.position) < 16.0)
        .map(|player| player.position)
}

pub(crate) fn dragon_breath_particle() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "type".into(),
            value: NbtSummaryValue::string("minecraft:dragon_breath"),
        },
        NbtSummaryEntry {
            name: "power".into(),
            value: NbtSummaryValue::Float(1.0_f32.to_bits()),
        },
    ])
}

fn instant_damage_two() -> bbb_protocol::packets::PotionContentsSummary {
    bbb_protocol::packets::PotionContentsSummary {
        custom_effects: vec![bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: bbb_protocol::mob_effect_types::VANILLA_MOB_EFFECT_INSTANT_DAMAGE_ID,
            amplifier: 1,
            duration: 1,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        }],
        ..Default::default()
    }
}
