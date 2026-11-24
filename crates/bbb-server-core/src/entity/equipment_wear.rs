use crate::world::simulation_level::LevelCtx;
use bbb_protocol::codec::Encoder;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::VANILLA_ITEM_ARMADILLO_SCUTE_ID;
use bbb_protocol::packets::{LevelParticles, ParticlePayload, Vec3d};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ITEM_ID;
use bbb_protocol::NbtSummaryValue;

use crate::entity::store::PendingEntitySound;
use crate::entity::{Entity, EntityStore};

pub(crate) fn hurt_and_break(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    slot: EquipmentSlot,
    amount: i32,
) -> Result<bool, String> {
    let Some(stack) = entity
        .living()
        .and_then(|living| living.item_by_slot(slot))
        .filter(|stack| !stack.is_empty())
        .cloned()
    else {
        return Ok(false);
    };
    let Some((damage, max_damage)) = crate::entity::item_components::damage_and_max(&stack)? else {
        return Ok(false);
    };
    let amount = if amount > 0
        && !crate::entity::enchanted_damage::stack_enchantments(
            &stack,
            store.enchantment_registry(),
        )
        .is_empty()
    {
        level.durability_change(&stack, amount)?
    } else {
        amount
    };
    if amount == 0 {
        return Ok(false);
    }
    let damage = damage.wrapping_add(amount).clamp(0, max_damage.max(0));
    let publication = level.item_stack_resolver();
    let mut damaged = stack
        .with_component_nbt(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            Some(&NbtSummaryValue::Int(damage)),
            publication,
        )
        .map_err(|error| error.to_string())?;
    let broke = damage >= max_damage;
    if broke {
        damaged.shrink(1);
    }
    entity
        .living_mut()
        .ok_or("the equipment owner is not living")?
        .update_item_slot(slot, damaged)
        .ok_or("the original equipment slot disappeared")?;
    if broke {
        crate::entity::daylight_burn::on_equipped_item_broken(entity, store, level, slot)?;
    }
    Ok(broke)
}

pub(crate) fn wolf_armor_absorb(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    damage: f32,
) -> Result<(), String> {
    let Some(before) = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::Body))
        .cloned()
    else {
        return Ok(());
    };
    let before = wolf_armor_crackiness(
        crate::entity::item_components::damage_value(&before)?,
        crate::entity::item_components::max_damage(&before)?,
    );
    // `Mth.ceil`, whose `(int)` saturates before the `+ 1` wraps.
    let truncated = damage as i32;
    let amount = if damage > truncated as f32 {
        truncated.wrapping_add(1)
    } else {
        truncated
    };
    hurt_and_break(entity, store, level, EquipmentSlot::Body, amount)?;
    let after = match entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::Body))
    {
        Some(stack) if !stack.is_empty() => crate::entity::item_components::damage_and_max(stack)?
            .map_or(Crackiness::None, |(damage, max)| {
                wolf_armor_crackiness(damage, max)
            }),
        _ => Crackiness::None,
    };
    if before == after {
        return Ok(());
    }
    let position = entity.position();
    store.play_entity_sound(PendingEntitySound {
        position,
        resource_id: "minecraft:item.wolf_armor.crack".to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch: 1.0,
    });
    // `new ItemParticleOption(ParticleTypes.ITEM, Items.ARMADILLO_SCUTE)` over
    // `ItemStackTemplate.STREAM_CODEC`: item, count 1, empty patch.
    let mut options = Encoder::default();
    options.write_var_i32(VANILLA_ITEM_ARMADILLO_SCUTE_ID);
    options.write_var_i32(1);
    options.write_var_i32(0);
    options.write_var_i32(0);
    store.send_particles(LevelParticles {
        override_limiter: false,
        always_show: false,
        position: Vec3d {
            x: position.x,
            y: position.y + 1.0,
            z: position.z,
        },
        offset: Vec3d {
            x: 0.2,
            y: 0.1,
            z: 0.2,
        },
        max_speed: 0.1,
        count: 20,
        particle: ParticlePayload {
            particle_type_id: VANILLA_PARTICLE_TYPE_ITEM_ID,
            raw_options: options.into_inner(),
        },
    });
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Crackiness {
    None,
    Low,
    Medium,
    High,
}

fn wolf_armor_crackiness(damage: i32, max_damage: i32) -> Crackiness {
    let fraction = (max_damage - damage) as f32 / max_damage as f32;
    if fraction < 0.32 {
        Crackiness::High
    } else if fraction < 0.69 {
        Crackiness::Medium
    } else if fraction < 0.95 {
        Crackiness::Low
    } else {
        Crackiness::None
    }
}
