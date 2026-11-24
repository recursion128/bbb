//! `Goat.mobInteract` (`Goat.java:211-218`) after `Entity.interact` PASSed.

use bbb_protocol::item_types::{VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_MILK_BUCKET_ID};
use bbb_protocol::packets::{ItemStackSummary, SoundSource};

use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

/// `SoundEvents.GOAT_MILK` (`SoundEvents.java:733`).
const GOAT_MILK_SOUND: &str = "minecraft:entity.goat.milk";

/// `SoundEvents.GOAT_SCREAMING_MILK` (`SoundEvents.java:742`).
const GOAT_SCREAMING_MILK_SOUND: &str = "minecraft:entity.goat.screaming.milk";

fn is_goat(entity: &Entity) -> bool {
    matches!(
        entity.data(),
        EntityData::GeneralLiving(data) if data.goat_retained().is_some()
    )
}

fn is_baby(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.ageable_retained().is_some_and(|age| age.is_baby())
}

fn milking_sound(entity: &Entity) -> &'static str {
    // `Goat.getMilkingSound` (`Goat.java:141-143`).
    let screaming = match entity.data() {
        EntityData::GeneralLiving(data) => {
            data.goat_retained().is_some_and(|goat| goat.is_screaming())
        }
        _ => false,
    };
    if screaming {
        GOAT_SCREAMING_MILK_SOUND
    } else {
        GOAT_MILK_SOUND
    }
}

/// `true` is SUCCESS (`Goat.java:217`). `false` falls through to
/// `super.mobInteract` (`:220`).
pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player_entity_id: i32,
    player_position: bbb_protocol::packets::Vec3d,
    hand_stack: &mut ItemStackSummary,
    infinite_materials: bool,
) -> bool {
    if hand_stack.item_id != Some(VANILLA_ITEM_BUCKET_ID) {
        return false;
    }
    let Some(target) = store.get(target_id) else {
        return false;
    };
    if !is_goat(target) || is_baby(target) {
        return false;
    }
    // Goat.java:214; Player.java:397-399: player.playSound at player pos, ungated by goat Silent.
    let resource_id = milking_sound(target).to_owned();
    store.play_entity_sound_except(
        player_entity_id,
        PendingEntitySound {
            position: player_position,
            resource_id,
            source: SoundSource::Players,
            volume: 1.0,
            pitch: 1.0,
        },
    );
    if !infinite_materials && hand_stack.count == 1 {
        *hand_stack = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_MILK_BUCKET_ID),
            count: 1,
            component_patch: Default::default(),
        };
    }
    true
}
