//! Actual inventory throws and the resulting world-owned advancement facts.

use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;

use crate::entity::{Entity, EntityCommand, EntityData, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;

// Allay.java:86-88.
const THROW_PITCHES: [f32; 16] = [
    0.5625, 0.625, 0.75, 0.9375, 1.0, 1.0, 1.125, 1.25, 1.5, 1.875, 2.0, 2.25, 2.5, 3.0, 3.75, 4.0,
];

// GoAndGiveItemsToTarget.java:69-75.
pub(crate) fn deliver(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let Some(runtime) = store.mob_ai(entity.id()) else {
        return;
    };
    let Some(target) = super::deposit_target(&runtime.target_view(entity, store, level)) else {
        return;
    };
    let eye = Vec3d {
        y: entity.position().y + f64::from(entity.eye_height()),
        ..entity.position()
    };
    if eye.distance_to_sqr(target.position) >= 9.0 {
        return;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(inventory) = data.internal_inventory_mut() else {
        return;
    };
    let Some(carried) = inventory.item(0) else {
        return;
    };
    let item = carried.with_count(1);
    let remaining = carried.count() - 1;
    inventory.update_count(0, remaining);
    crate::entity::item::throw_item_with_velocity(
        entity,
        store,
        Vec3d {
            y: target.position.y + 1.0,
            ..target.position
        },
        item.clone(),
        Vec3d {
            x: f64::from(0.2_f32),
            y: f64::from(0.3_f32),
            z: f64::from(0.2_f32),
        },
        f64::from(0.2_f32),
    );
    super::on_item_thrown(entity);
    if let Some(player) = target.liked_player {
        store.queue_command(EntityCommand::AllayDropItemOnBlock {
            player,
            pos: BlockPos {
                y: target.block_pos.y - 1,
                ..target.block_pos
            },
            item,
        });
    }
    // AllayAi.java:158-164.
    if !entity.is_silent() && level.game_time() % 7 == 0 && store.random().next_double() < 0.9 {
        let pitch = THROW_PITCHES[store.random().next_int(THROW_PITCHES.len() as i32) as usize];
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:entity.allay.item_thrown".into(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch,
        });
    }
}

pub(crate) fn drop_carried_items(entity: &mut Entity, store: &mut EntityStore) {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(inventory) = data.internal_inventory_mut() else {
        return;
    };
    let dropped = inventory.remove_all_items();
    for (_, stack) in dropped {
        crate::entity::item::throw_item(entity, store, entity.position(), stack);
    }
}
