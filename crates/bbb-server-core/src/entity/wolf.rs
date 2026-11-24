//! A wet wolf shaking itself dry (`Wolf.java:299-358`).

use crate::connection::ClientboundPlayPacket;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::EntityEvent;

const START_SHAKE_EVENT: i8 = 8;
const CANCEL_SHAKE_EVENT: i8 = 56;
// `shakeAnim` grows by 0.05 a tick and the shake ends once its previous value reaches 2.0.
const SHAKE_TICKS: u8 = 41;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WolfShake {
    wet: bool,
    shaking_ticks: Option<u8>,
}

/// `Wolf.aiStep` (`:299-306`) followed by the rest of `Wolf.tick` (`:315-357`).
pub(crate) fn tick(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) {
    if !entity.is_alive_with_health() {
        return;
    }
    let path_finding = store
        .mob_ai(entity.id())
        .is_some_and(|runtime| runtime.ground_path().is_some());
    let wet_now = entity.is_in_water() || entity.is_in_rain(level);
    let on_ground = entity.on_ground();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(shake) = data.wolf_shake_mut() else {
        return;
    };
    let mut events = [None, None];
    if shake.wet && shake.shaking_ticks.is_none() && !path_finding && on_ground {
        shake.shaking_ticks = Some(0);
        events[0] = Some(START_SHAKE_EVENT);
    }
    let mut sound = false;
    if wet_now {
        shake.wet = true;
        if shake.shaking_ticks.take().is_some() {
            events[1] = Some(CANCEL_SHAKE_EVENT);
        }
    } else if let Some(ticks) = shake.shaking_ticks {
        sound = ticks == 0;
        if ticks + 1 >= SHAKE_TICKS {
            *shake = WolfShake::default();
        } else {
            shake.shaking_ticks = Some(ticks + 1);
        }
    }
    for event_id in events.into_iter().flatten() {
        store.broadcast_to_tracking(
            entity.id(),
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: entity.id(),
                event_id,
            }),
        );
    }
    if sound {
        if !entity.is_silent() {
            let random = store.random();
            let pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
            store.play_entity_sound(crate::entity::store::PendingEntitySound {
                position: entity.position(),
                resource_id: "minecraft:entity.wolf.shake".to_owned(),
                source: crate::entity::living::living_sound_source(entity),
                // Wolf.java:294-296.
                volume: 0.4,
                pitch,
            });
        }
        level.game_event_at(
            VanillaGameEvent::ENTITY_ACTION,
            entity.position(),
            Some(entity.id()),
        );
    }
}
