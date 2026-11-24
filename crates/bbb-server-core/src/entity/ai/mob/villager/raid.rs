//! Villager actions that reach the live level and entity store during raids.

use crate::entity::ai::brain::{Behavior, BehaviorHooks, BrainTick, BrainTickContext};
use crate::entity::ai::{Activity, MemoryModuleType, MemoryValue};
use crate::entity::{Entity, EntityCommand, EntityStore};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn tick_raid_actions(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) {
    if !crate::entity::villager::is_villager(entity) {
        return;
    }
    let Some(brain) = store
        .mob_ai(entity.id())
        .and_then(|runtime| runtime.brain.as_ref())
    else {
        return;
    };
    if brain.is_active(Activity::CELEBRATE) {
        if brain.running_behaviors().contains(&CELEBRATE_BEHAVIOR) {
            celebrate(entity, store, level);
        }
        return;
    }
    if !brain.is_active(Activity::PRE_RAID) {
        return;
    }
    let Some(MemoryValue::GlobalPos(meeting)) = brain.memory(MemoryModuleType::MeetingPoint) else {
        return;
    };
    // `RingBell.create`.
    let pos = meeting.pos;
    let at = entity.block_position();
    let distance = (f64::from(pos.x) - f64::from(at.x)).powi(2)
        + (f64::from(pos.y) - f64::from(at.y)).powi(2)
        + (f64::from(pos.z) - f64::from(at.z)).powi(2);
    if distance >= 9.0 || store.random().next_float() <= 0.95 {
        return;
    }
    let state = level.block_state(pos);
    if crate::block_behavior::bell::is_bell(state) {
        store.queue_command(EntityCommand::RingBell {
            pos,
            direction: crate::block_behavior::bell::facing(state),
            source_entity_id: entity.id(),
        });
    }
}

// `MoveToSkySeeingSpot.hasNoBlocksAbove`.
pub(crate) fn has_no_blocks_above(
    level: &LevelCtx<'_, '_>,
    pos: bbb_protocol::packets::BlockPos,
    body_y: f64,
) -> bool {
    level.can_see_sky(pos)
        && level
            .motion_blocking_height(pos.x, pos.z)
            .is_some_and(|height| f64::from(height) <= body_y)
}

const CELEBRATE_BEHAVIOR: &str = "CelebrateVillagersSurvivedRaid";

/// Only the start needs open sky;
/// the running behaviour keeps cheering under a roof for its 600 ticks.
#[derive(Clone)]
struct CelebrateVillagersSurvivedRaid;

impl BehaviorHooks for CelebrateVillagersSurvivedRaid {
    fn extra_start_conditions(
        &mut self,
        tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
    ) -> bool {
        tick.world_mut().body_has_no_blocks_above()
    }

    fn can_still_use(
        &self,
        _tick: &mut BrainTick<'_>,
        _context: &mut BrainTickContext,
        _timestamp: i64,
    ) -> bool {
        true
    }

    fn debug_string(&self) -> &'static str {
        CELEBRATE_BEHAVIOR
    }
}

pub(crate) fn celebrate_villagers_survived_raid() -> Behavior {
    Behavior::with_duration(Vec::new(), 600, 600, CelebrateVillagersSurvivedRaid)
}

fn celebrate(entity: &Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    // `CelebrateVillagersSurvivedRaid.tick`.
    if store.random().next_int(100) == 0 {
        crate::entity::villager::play_body_sound(
            entity,
            store,
            "minecraft:entity.villager.celebrate",
        );
    }
    if store.random().next_int(200) != 0
        || !has_no_blocks_above(level, entity.block_position(), entity.position().y)
    {
        return;
    }
    let color = bbb_protocol::builtin_enum_component_values::vanilla_dye_color_firework_color(
        store.random().next_int(16),
    );
    let flight = store.random().next_int(3);
    let Some(item) = celebration_firework(color, flight) else {
        return;
    };
    crate::entity::firework_rocket::spawn_launched_firework(
        store,
        bbb_protocol::packets::Vec3d {
            y: entity.position().y + f64::from(entity.eye_height()),
            ..entity.position()
        },
        item,
        entity.uuid(),
        None,
    );
}

// `CelebrateVillagersSurvivedRaid.getFirework`.
fn celebration_firework(
    color: i32,
    flight: i32,
) -> Option<bbb_server_types::item_stack::RuntimeItemStack> {
    use bbb_protocol::item_component_value_types::FireworkExplosionShapeSummary;
    use bbb_protocol::packets::{DataComponentPatchSummary, FireworkExplosionSummary};
    let patch = DataComponentPatchSummary {
        added_type_ids: vec![
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
        ],
        fireworks_flight_duration: Some(flight),
        fireworks_explosions: vec![FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Burst,
            colors: vec![color],
            fade_colors: Vec::new(),
            has_trail: false,
            has_twinkle: false,
        }],
        ..Default::default()
    };
    crate::entity::item::plain_stack("minecraft:firework_rocket", 1)?
        .with_component_patch(&patch, &bbb_registry_codec::UnavailableHolderResolver)
        .ok()
}
