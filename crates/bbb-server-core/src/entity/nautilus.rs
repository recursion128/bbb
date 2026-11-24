//! Home maintenance and riding dash cues shared by both nautilus families.

use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;

use super::{Entity, EntityData, EntityRegistryTagView, EntityStore, PendingEntitySound};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NautilusDash {
    pub(crate) dashing: bool,
    cooldown: u8,
}

impl NautilusDash {
    pub(crate) fn new(entity_type_id: i32) -> Option<Self> {
        matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
        )
        .then(Self::default)
    }
}

fn state(entity: &mut Entity) -> Option<&mut NautilusDash> {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return None;
    };
    data.nautilus_dash_mut()
}

// AbstractNautilus.java:318-320; AbstractNautilus.java:354-358.
pub(crate) fn start_jump(
    entity: &mut Entity,
    store: &mut EntityStore,
    tags: &EntityRegistryTagView,
) -> bool {
    if super::living::valid_equipment::has_valid_item_with_registry_tags(
        entity,
        tags,
        EquipmentSlot::Saddle,
    ) != Some(true)
    {
        return false;
    }
    let Some(dash) = state(entity) else {
        return false;
    };
    if !dash.dashing {
        dash.dashing = true;
        if dash.cooldown == 0 {
            dash.cooldown = 40;
        }
        entity.mark_synched_dirty(VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID);
    }
    play_sound(entity, store, false);
    true
}

// AbstractNautilus.java:301-310; AbstractNautilus.java:366-372.
pub(crate) fn tick(entity: &mut Entity, store: &mut EntityStore) {
    check_home(entity, store);
    let Some(dash) = state(entity) else {
        return;
    };
    let cleared = dash.dashing && dash.cooldown < 35;
    if cleared {
        dash.dashing = false;
    }
    let ready = dash.cooldown > 0 && {
        dash.cooldown -= 1;
        dash.cooldown == 0
    };
    if cleared {
        entity.mark_synched_dirty(VANILLA_ABSTRACT_NAUTILUS_DASH_DATA_ID);
    }
    if ready {
        play_sound(entity, store, true);
    }
}

// AbstractNautilus.java:241-257.
fn check_home(entity: &mut Entity, store: &EntityStore) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    if !matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_NAUTILUS_ID | VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID
    ) || data.is_no_ai()
        || !data.is_tame()
        || !entity.is_alive()
        || super::leash::is_leashed(entity)
        || !store.passengers(entity.id()).is_empty()
    {
        return;
    }
    let saddled = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::Saddle))
        .is_some();
    let radius = if !data.is_baby() && !saddled { 32 } else { 16 };
    let position = entity.block_position();
    if entity.mob_home().is_none_or(|(home, old_radius)| {
        let dx = i64::from(home.x) - i64::from(position.x);
        let dy = i64::from(home.y) - i64::from(position.y);
        let dz = i64::from(home.z) - i64::from(position.z);
        old_radius != radius || dx * dx + dy * dy + dz * dz >= i64::from(radius + 8).pow(2)
    }) {
        entity.set_mob_home(position, radius);
    }
}

fn play_sound(entity: &Entity, store: &mut EntityStore, ready: bool) {
    if entity.is_silent() {
        return;
    }
    // Nautilus.java:91-98; ZombieNautilus.java:102-109.
    // SoundEvents.java:1068-1071; SoundEvents.java:1812-1815.
    let family = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_NAUTILUS_ID {
        "nautilus"
    } else {
        "zombie_nautilus"
    };
    let action = if ready { "dash_ready" } else { "dash" };
    let land = if entity.is_under_water() { "" } else { "_land" };
    let baby = matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let pitch = super::living::voice_pitch(store, baby);
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: format!("minecraft:entity.{family}.{action}{land}"),
        source: super::living::living_sound_source(entity),
        volume: 1.0,
        pitch,
    });
}
