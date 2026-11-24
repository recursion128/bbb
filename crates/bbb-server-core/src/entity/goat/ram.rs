//! Goat ram cooldowns, impacts and dropped horns on the live entity.

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_GOAT_LEFT_HORN_DATA_ID, VANILLA_GOAT_RIGHT_HORN_DATA_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_GOAT_HORN_ID;
use bbb_protocol::packets::{BlockPos, EntityEvent, ItemStackSummary, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryValue, PersistentRegistryTagMembers};
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::mob::brain_runtime::hurt_entity_from_body;
use crate::entity::ai::{MemoryModuleType, MemoryValue, PackedMemory};
use crate::entity::general_living::set_spawner_brain_memory;
use crate::entity::level::block_state_tag_membership;
use crate::entity::{Entity, EntityCommand, EntityData, EntityStore, PendingEntitySound};
use crate::level_event::{PendingHurtEntity, PendingPostAttack};
use crate::world::simulation_level::LevelCtx;

// RamTarget.java:114-118.
pub(crate) fn has_horn_breaking_block(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    wanted: Vec3d,
) -> bool {
    let mut movement = entity.delta_movement();
    if entity.horizontal_collision() && movement.x == 0.0 && movement.z == 0.0 {
        // Collision clipping already cleared the blocked velocity.
        let position = entity.position();
        movement = Vec3d {
            x: wanted.x - position.x,
            y: 0.0,
            z: wanted.z - position.z,
        };
    }
    let direction = Vec3d { y: 0.0, ..movement }.normalize();
    let position = entity.position().plus(direction);
    let facing = BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    };
    [
        facing,
        BlockPos {
            y: facing.y + 1,
            ..facing
        },
    ]
    .into_iter()
    .any(|pos| {
        level.has_ticking_column_at(pos)
            && block_state_tag_membership(
                level,
                level.block_state(pos),
                "minecraft:snaps_goat_horn",
            )
    })
}

pub(crate) fn finish_ram(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    failed: bool,
    snap_horn: bool,
    impact: bool,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(goat) = data.goat_retained() else {
        return;
    };
    let screaming = goat.is_screaming();
    // GoatAi.java:48-49; PrepareRamNearestTarget.java:78-82; RamTarget.java:121-124.
    let minimum = if screaming { 100 } else { 600 };
    let cooldown = minimum
        + if failed {
            0
        } else {
            level
                .level_random()
                .next_int(if screaming { 201 } else { 5401 })
        };
    set_spawner_brain_memory(
        entity,
        PackedMemory {
            memory_type: MemoryModuleType::RamCooldownTicks,
            value: MemoryValue::Int(cooldown),
            time_to_live: None,
        },
    );
    super::set_lowering_head(entity, false);
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: entity.id(),
            event_id: 59,
        }),
    );
    if impact {
        play_sound(
            entity,
            store,
            if screaming {
                "minecraft:entity.goat.screaming.ram_impact"
            } else {
                "minecraft:entity.goat.ram_impact"
            },
        );
    }
    if snap_horn && drop_horn(entity, store, level) {
        play_sound(entity, store, "minecraft:entity.goat.horn_break");
    }
}

// RamTarget.java:79-99.
pub(crate) fn apply_ram_hit(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    knockback: f64,
    direction: Vec3d,
) {
    let Some(living) = entity.living() else {
        return;
    };
    let damage = living.attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32;
    if let Some(player) = level.player_by_entity_id(target_id) {
        store.queue_command(EntityCommand::GoatRamPlayer {
            goat: entity.uuid(),
            player: player.uuid,
            origin: entity.position(),
            damage,
            knockback,
            direction,
        });
        return;
    }
    // DamageTypes.java:37.
    const DAMAGE_TYPE: &str = "minecraft:mob_attack_no_aggro";
    if hurt_entity_from_body(entity, store, level, target_id, DAMAGE_TYPE, damage) == Some(true) {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.post_attack(PendingPostAttack {
            victim_entity_id: target_id,
            damage_type: DAMAGE_TYPE,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
        });
    }
    store.with_entity_mut(target_id, |target, store| {
        if target.creaking_can_move() {
            crate::entity::living::knockback(target, store, knockback, direction.x, direction.z);
        }
    });
}

// Goat.java:304-332.
fn drop_horn(entity: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    let Some(goat) = data.goat_retained() else {
        return false;
    };
    if data.is_baby() || !(goat.left_horn || goat.right_horn) {
        return false;
    }
    let left = goat.left_horn && (!goat.right_horn || level.level_random().next_boolean());
    let Some(item) = create_horn(entity, goat.is_screaming(), level) else {
        return false;
    };
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    let goat = data.goat_retained_mut().unwrap();
    if left {
        goat.left_horn = false;
    } else {
        goat.right_horn = false;
    }
    entity.mark_synched_dirty(if left {
        VANILLA_GOAT_LEFT_HORN_DATA_ID
    } else {
        VANILLA_GOAT_RIGHT_HORN_DATA_ID
    });
    let random = level.level_random();
    let velocity = Vec3d {
        x: f64::from(-0.2_f32 + random.next_float() * 0.4_f32),
        y: f64::from(0.3_f32 + random.next_float() * 0.4_f32),
        z: f64::from(-0.2_f32 + random.next_float() * 0.4_f32),
    };
    crate::entity::item::spawn_dispensed_item_entity(store, entity.position(), item, velocity);
    true
}

// Goat.java:91-99.
fn create_horn(
    entity: &Entity,
    screaming: bool,
    level: &LevelCtx<'_, '_>,
) -> Option<RuntimeItemStack> {
    let item = RuntimeItemStack::from_summary(ItemStackSummary {
        item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
        count: 1,
        component_patch: Default::default(),
    });
    let resolver = level.item_stack_resolver();
    let tag = if screaming {
        "minecraft:screaming_goat_horns"
    } else {
        "minecraft:regular_goat_horns"
    };
    let PersistentRegistryTagMembers::Present(members) =
        resolver.tag_members("minecraft:instrument", tag)
    else {
        return Some(item);
    };
    if members.is_empty() {
        return Some(item);
    }
    // Both horns of one goat carry the same instrument.
    let uuid = entity.uuid().as_u128();
    let index = (((uuid >> 64) as u64 ^ uuid as u64) % members.len() as u64) as usize;
    item.with_component_nbt(
        VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
        Some(&NbtSummaryValue::string(members[index].clone())),
        resolver,
    )
    .ok()
}

fn play_sound(entity: &Entity, store: &mut EntityStore, resource_id: &str) {
    if entity.is_silent() {
        return;
    }
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_owned(),
        source: SoundSource::Neutral,
        volume: 1.0,
        pitch: 1.0,
    });
}
