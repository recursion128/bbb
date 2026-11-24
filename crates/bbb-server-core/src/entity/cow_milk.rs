//! `AbstractCow.mobInteract` (`AbstractCow.java:89-99`) after `Entity.interact` PASSed.

use bbb_protocol::item_types::{VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_MILK_BUCKET_ID};
use bbb_protocol::packets::{ItemStackSummary, SoundSource};

use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

/// `SoundEvents.COW_MILK` (`SoundEvents.java:449`).
const COW_MILK_SOUND: &str = "minecraft:entity.cow.milk";

/// `MushroomCow` extends `AbstractCow` and falls through to super for a bucket
/// (`MushroomCow.java:167`).
fn is_abstract_cow(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    matches!(
        data.entity_resource_id(),
        "minecraft:cow" | "minecraft:mooshroom"
    )
}

fn is_baby(entity: &Entity) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.ageable_retained().is_some_and(|age| age.is_baby())
}

/// `true` is SUCCESS (`:95`). `false` is `super.mobInteract` (`:97`).
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
    if !is_abstract_cow(target) || is_baby(target) {
        return false;
    }
    // AbstractCow.java:92; Player.java:397-399: player.playSound excludes the actor.
    store.play_entity_sound_except(
        player_entity_id,
        PendingEntitySound {
            position: player_position,
            resource_id: COW_MILK_SOUND.to_owned(),
            source: SoundSource::Players,
            volume: 1.0,
            pitch: 1.0,
        },
    );
    // Count==1 survival replace and creative keep; leftover add/drop for
    // count>1 is the world's `finish_leash_interaction` arm (`ItemUtils.java:16-42`).
    if !infinite_materials && hand_stack.count == 1 {
        *hand_stack = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_MILK_BUCKET_ID),
            count: 1,
            component_patch: Default::default(),
        };
    }
    true
}
