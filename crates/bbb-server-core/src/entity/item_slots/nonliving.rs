use super::SlotWriteEffects;
use crate::entity::{Entity, EntityData};
use bbb_server_types::item_stack::RuntimeItemStack;

pub(super) fn read(entity: &Entity, slot: i32) -> Option<RuntimeItemStack> {
    if slot != 0 {
        return None;
    }
    Some(match entity.data() {
        EntityData::Item(data) => data.item().clone(),
        EntityData::ItemFrame(data) | EntityData::GlowItemFrame(data) => data
            .item()
            .cloned()
            .unwrap_or_else(|| crate::entity::item::empty_stack().expect("empty stack")),
        EntityData::ItemDisplay(data) => data.item().clone(),
        EntityData::Arrow(data) => data.item().clone(),
        EntityData::SpectralArrow(data) => data.item().clone(),
        EntityData::Trident(data) => data.item().clone(),
        EntityData::LargeFireball(data) => data.item().clone(),
        EntityData::SmallFireball(data) => data.item().clone(),
        _ => return None,
    })
}

pub(super) fn write(
    entity: &mut Entity,
    slot: i32,
    stack: RuntimeItemStack,
) -> Result<Option<SlotWriteEffects>, String> {
    use bbb_protocol::entity_synched_data::{
        VANILLA_ARROW_EFFECT_COLOR_DATA_ID, VANILLA_FIREBALL_ITEM_DATA_ID,
        VANILLA_ITEM_ENTITY_ITEM_DATA_ID,
    };
    if slot != 0 {
        return Ok(None);
    }
    let mut effects = SlotWriteEffects::default();
    let default = |stack: RuntimeItemStack, resource: &str| {
        if stack.is_empty() {
            crate::entity::item::plain_stack(resource, 1).ok_or_else(|| format!("{resource} stack"))
        } else {
            Ok(stack)
        }
    };
    match &mut entity.data {
        EntityData::Item(data) => {
            data.set_item(stack);
            entity.mark_synched_dirty(VANILLA_ITEM_ENTITY_ITEM_DATA_ID);
        }
        EntityData::ItemFrame(_) | EntityData::GlowItemFrame(_) => {
            let (sound, attached) = crate::entity::hanging::set_slot_item(entity, stack)?;
            effects.sound = sound;
            effects.frame_output = Some(attached);
        }
        EntityData::ItemDisplay(data) => {
            data.replace_slot_item(stack);
            entity.mark_synched_dirty(crate::entity::display::ITEM_DISPLAY_ITEM_STACK_DATA_ID);
        }
        EntityData::Arrow(data) => {
            let stack = default(stack, "minecraft:arrow")?;
            let color = crate::entity::thrown_potion::arrow_item_color(&stack)?;
            let changed = color != data.effect_color();
            data.replace_slot_item(stack);
            data.set_effect_color(color);
            if changed {
                entity.mark_synched_dirty(VANILLA_ARROW_EFFECT_COLOR_DATA_ID);
            }
        }
        EntityData::SpectralArrow(data) => {
            data.replace_slot_item(default(stack, "minecraft:spectral_arrow")?)
        }
        EntityData::Trident(data) => data.replace_slot_item(default(stack, "minecraft:trident")?),
        EntityData::LargeFireball(data) => {
            let stack = default(stack, "minecraft:fire_charge")?.with_count(1);
            data.replace_slot_item(stack);
            entity.mark_synched_dirty(VANILLA_FIREBALL_ITEM_DATA_ID);
        }
        EntityData::SmallFireball(data) => {
            let stack = default(stack, "minecraft:fire_charge")?.with_count(1);
            data.replace_slot_item(stack);
            entity.mark_synched_dirty(VANILLA_FIREBALL_ITEM_DATA_ID);
        }
        _ => return Ok(None),
    }
    Ok(Some(effects))
}
