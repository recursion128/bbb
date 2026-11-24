//! Threat detection and the armadillo's rolling posture.

use super::{ArmadilloPose, ArmadilloState};
use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::mob::target::TargetView;
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::{Entity, EntityData, EntityStore, StoredEntityDamageSource};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_ARMADILLO_STATE_DATA_ID;
use bbb_protocol::packets::{EntityEvent, SoundSource, Vec3d};

fn state(body: &Entity) -> Option<&ArmadilloState> {
    match body.data() {
        EntityData::GeneralLiving(data) => data.armadillo_state(),
        _ => None,
    }
}

fn state_mut(body: &mut Entity) -> Option<&mut ArmadilloState> {
    match &mut body.data {
        EntityData::GeneralLiving(data) => data.armadillo_state_mut(),
        _ => None,
    }
}

fn change_pose(body: &mut Entity, pose: ArmadilloPose) {
    let tick = body.tick_count();
    let Some(state) = state_mut(body) else {
        return;
    };
    if state.pose != pose {
        state.pose = pose;
        state.state_since = tick;
        body.mark_synched_dirty(VANILLA_ARMADILLO_STATE_DATA_ID);
    }
}

fn sound(body: &Entity, store: &mut EntityStore, resource_id: &str, voiced: bool) {
    if body.is_silent() {
        return;
    }
    let baby = matches!(body.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let pitch = if voiced {
        crate::entity::living::voice_pitch(store, baby)
    } else {
        1.0
    };
    store.play_sound_from_entity_at(
        body.id(),
        body.position(),
        resource_id,
        SoundSource::Neutral,
        1.0,
        pitch,
    );
}

// Armadillo.java:331-332.
fn can_stay_rolled(body: &Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    let sensing = Default::default();
    let targets = TargetView::new(body, store, level, &sensing);
    !body.is_in_water()
        && !body.is_in_lava()
        && !crate::entity::leash::is_leashed(body)
        && store.vehicle(body.id()).is_none()
        && store.passengers(body.id()).is_empty()
        && !store.mob_ai(body.id()).is_some_and(|ai| ai.is_panicking())
        && !crate::entity::ai::mob::panic::should_panic(
            crate::entity::ai::mob::panic::PanicKind::AnimalEnvironmental,
            &targets,
        )
}

fn refresh_danger(body: &mut Entity) {
    crate::entity::set_spawner_brain_memory(
        body,
        PackedMemory {
            memory_type: MemoryModuleType::DangerDetectedRecently,
            value: MemoryValue::Bool(true),
            time_to_live: Some(80),
        },
    );
}

fn clear_danger(body: &mut Entity) {
    if let EntityData::GeneralLiving(data) = &mut body.data {
        data.retained_brain_mut()
            .memories
            .retain(|entry| entry.memory_type != MemoryModuleType::DangerDetectedRecently);
    }
}

// Armadillo.java:235-246; NearestLivingEntitySensor.java:18-21.
fn threat_nearby(body: &Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    let range = body.living().map_or(16.0, |living| {
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
    });
    let area = body
        .bounding_box()
        .inflate(range.min(7.0), range.min(2.0), range.min(7.0));
    let sensing = Default::default();
    let view = TargetView::new(body, store, level, &sensing);
    let attacker = view.last_hurt_by_mob().map(|target| target.uuid());
    for id in store.ids_in_box_sections(area) {
        let Some(other) = store.get(id) else {
            continue;
        };
        if id != body.id()
            && other.living().is_some()
            && other.is_alive_with_health()
            && other.bounding_box().intersects(area)
            && (attacker == Some(other.uuid())
                || level.registry_tag_membership(
                    "minecraft:entity_type",
                    "minecraft:undead",
                    other.data().entity_resource_id(),
                ))
        {
            return true;
        }
    }
    level.players().iter().any(|nearby| {
        !nearby.dead_or_dying
            && nearby.bounding_box().intersects(area)
            && (attacker == Some(nearby.uuid)
                || store.player(nearby.entity_id).is_some_and(|player| {
                    !player.is_spectator()
                        && (player.is_sprinting() || store.vehicle(nearby.entity_id).is_some())
                }))
    })
}

// Armadillo.java:263-279.
fn roll_up(body: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if super::is_scared(body) {
        return;
    }
    store.stop_mob_navigation(body.id());
    body.set_delta_movement(Vec3d::default());
    if let Some(living) = body.living_mut() {
        crate::entity::living::apply_immobile(living);
    }
    if let EntityData::GeneralLiving(data) = &mut body.data {
        if let Some(animal) = data.animal_retained_mut() {
            animal.reset_love();
        }
    }
    change_pose(body, ArmadilloPose::Rolling);
    level.game_event_at(
        VanillaGameEvent::ENTITY_ACTION,
        body.position(),
        Some(body.id()),
    );
    sound(body, store, "minecraft:entity.armadillo.roll", true);
}

fn roll_out(body: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if !super::is_scared(body) {
        return;
    }
    change_pose(body, ArmadilloPose::Idle);
    level.game_event_at(
        VanillaGameEvent::ENTITY_ACTION,
        body.position(),
        Some(body.id()),
    );
    sound(
        body,
        store,
        "minecraft:entity.armadillo.unroll_finish",
        true,
    );
}

// Armadillo.java:282-302.
pub(crate) fn damage(body: &Entity, damage: f32) -> f32 {
    if super::is_scared(body) {
        (damage - 1.0) / 2.0
    } else {
        damage
    }
}

pub(crate) fn was_hurt(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    living_attacker: bool,
) {
    if state(body).is_none()
        || !body.is_alive_with_health()
        || matches!(body.data(), EntityData::GeneralLiving(data) if data.is_no_ai())
    {
        return;
    }
    if living_attacker {
        refresh_danger(body);
        if can_stay_rolled(body, store, level) {
            roll_up(body, store, level);
        }
    } else if level.registry_tag_membership(
        "minecraft:damage_type",
        "minecraft:panic_environmental_causes",
        &source.damage_type.resource_id,
    ) {
        clear_danger(body);
        roll_out(body, store, level);
    }
}

// SensorType.java:26-28; ArmadilloAi.java:137-171.
pub(crate) fn tick(body: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if state(body).is_none() || !body.is_alive_with_health() {
        return;
    }
    if let EntityData::GeneralLiving(data) = &mut body.data {
        data.retained_brain_mut().memories.retain_mut(|entry| {
            if entry.memory_type == MemoryModuleType::DangerDetectedRecently {
                if let Some(ttl) = &mut entry.time_to_live {
                    *ttl = ttl.saturating_sub(1);
                    return *ttl > 0;
                }
            }
            true
        });
    }
    if !can_stay_rolled(body, store, level) {
        clear_danger(body);
        roll_out(body, store, level);
        return;
    }
    let scan = state_mut(body).is_some_and(|state| {
        if state.scan_delay > 0 {
            state.scan_delay -= 1;
            false
        } else {
            state.scan_delay = 4;
            true
        }
    });
    if scan && threat_nearby(body, store, level) {
        refresh_danger(body);
    }
    let danger =
        crate::entity::spawner_brain_memory(body, MemoryModuleType::DangerDetectedRecently)
            .map(|memory| memory.time_to_live.unwrap_or(i64::MAX));
    let Some(remaining) = danger else {
        roll_out(body, store, level);
        return;
    };
    let pose = state(body).unwrap().pose;
    match pose {
        ArmadilloPose::Idle if body.on_ground() => roll_up(body, store, level),
        ArmadilloPose::Rolling
            if body
                .tick_count()
                .wrapping_sub(state(body).unwrap().state_since)
                > 10 =>
        {
            change_pose(body, ArmadilloPose::Scared);
            if body.on_ground() {
                sound(body, store, "minecraft:entity.armadillo.land", false);
            }
        }
        ArmadilloPose::Scared | ArmadilloPose::Unrolling => {
            let around = remaining > 75;
            let redraw = state(body).unwrap().danger_was_around != around;
            let next_peek = redraw.then(|| 150 + level.level_random().next_int(301) as u16);
            let state = state_mut(body).unwrap();
            state.danger_was_around = around;
            state.next_peek = next_peek.unwrap_or_else(|| state.next_peek.saturating_sub(1));
            if pose == ArmadilloPose::Scared {
                if state.next_peek == 0 && around && body.on_ground() {
                    store.broadcast_to_tracking(
                        body.id(),
                        ClientboundPlayPacket::EntityEvent(EntityEvent {
                            entity_id: body.id(),
                            event_id: 64,
                        }),
                    );
                    state_mut(body).unwrap().next_peek =
                        150 + level.level_random().next_int(301) as u16;
                }
                if remaining < 30 {
                    change_pose(body, ArmadilloPose::Unrolling);
                    sound(
                        body,
                        store,
                        "minecraft:entity.armadillo.unroll_start",
                        false,
                    );
                }
            } else if remaining > 30 {
                change_pose(body, ArmadilloPose::Scared);
            }
        }
        _ => {}
    }
}
