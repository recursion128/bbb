//! Pufferfish inflation and contact stings.

use super::{set_puff_state, FishLeafState};
use crate::entity::ai::mob::brain_runtime::hurt_entity_from_body;
use crate::entity::ai::mob::target::{conditions::TargetingConditions, Target, TargetView};
use crate::entity::living::mob_effects::add_active_effect_in_level;
use crate::entity::{ActiveMobEffect, Entity, EntityData, EntityStore};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_types::{vanilla_entity_type, VANILLA_ENTITY_TYPE_PUFFERFISH_ID};
use bbb_protocol::packets::SoundSource;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PuffRuntime {
    inflate: u16,
    deflate: u16,
}

pub(crate) fn puff_state(body: &Entity) -> i32 {
    match body.data() {
        EntityData::GeneralLiving(data) => match data.fish_leaf() {
            Some(FishLeafState::Pufferfish(state)) => state,
            _ => 0,
        },
        _ => 0,
    }
}

fn runtime_mut(body: &mut Entity) -> &mut PuffRuntime {
    let EntityData::GeneralLiving(data) = &mut body.data else {
        unreachable!("pufferfish is living")
    };
    &mut data.fish_state_mut().expect("pufferfish state").puff
}

// Pufferfish.java:34-40.
fn scary<'w>(targets: &TargetView<'_, '_, 'w>, target: &Target<'_, '_, 'w>) -> bool {
    TargetingConditions::for_non_combat()
        .ignore_line_of_sight()
        .ignore_invisibility_testing()
        .test(targets, target, |target| {
            !target.creative()
                && !targets.level().registry_tag_membership(
                    "minecraft:entity_type",
                    "minecraft:not_scary_for_pufferfish",
                    vanilla_entity_type(target.entity_type_id())
                        .map_or("", |kind| kind.resource_id),
                )
        })
}

fn sound(body: &Entity, store: &mut EntityStore, id: &str, voiced: bool) {
    if !body.is_silent() {
        let pitch = if voiced {
            crate::entity::living::voice_pitch(store, false)
        } else {
            1.0
        };
        store.play_sound_from_entity_at(
            body.id(),
            body.position(),
            id,
            SoundSource::Neutral,
            1.0,
            pitch,
        );
    }
}

pub(crate) fn tick(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    if body.entity_type_id() != VANILLA_ENTITY_TYPE_PUFFERFISH_ID || !body.is_alive_with_health() {
        return true;
    }
    if !matches!(body.data(), EntityData::GeneralLiving(data) if data.is_no_ai()) {
        if body.tick_count() % 5 == 0 {
            let sensing = Default::default();
            let targets = TargetView::new(body, store, level, &sensing);
            // Pufferfish.java:200-208.
            let area = body.bounding_box().inflate(2.0, 2.0, 2.0);
            let danger = targets
                .local_players()
                .any(|target| target.bounding_box().intersects(area) && scary(&targets, &target))
                || store.ids_in_box_sections(area).into_iter().any(|id| {
                    targets.facts(id).is_some_and(|target| {
                        target.bounding_box().intersects(area) && scary(&targets, &target)
                    })
                });
            let runtime = runtime_mut(body);
            if danger {
                if runtime.inflate == 0 {
                    runtime.inflate = 1;
                    runtime.deflate = 0;
                }
            } else {
                runtime.inflate = 0;
            }
        }
        // Pufferfish.java:98-119.
        let state = puff_state(body);
        let runtime = runtime_mut(body);
        let next = if runtime.inflate > 0 {
            let next = if state == 0 {
                Some(1)
            } else if state == 1 && runtime.inflate > 40 {
                Some(2)
            } else {
                None
            };
            runtime.inflate = runtime.inflate.saturating_add(1);
            next
        } else if state != 0 {
            let next = if state == 2 && runtime.deflate > 60 {
                Some(1)
            } else if state == 1 && runtime.deflate > 100 {
                Some(0)
            } else {
                None
            };
            runtime.deflate = runtime.deflate.saturating_add(1);
            next
        } else {
            None
        };
        if let Some(next) = next {
            sound(
                body,
                store,
                if next > state {
                    "minecraft:entity.puffer_fish.blow_up"
                } else {
                    "minecraft:entity.puffer_fish.blow_out"
                },
                true,
            );
            set_puff_state(body, next);
        }
    }
    let state = puff_state(body);
    if state <= 0 {
        return true;
    }
    // Pufferfish.java:129-143. NoAI freezes inflation, but not contact damage.
    let area = body.bounding_box().inflate(0.3, 0.3, 0.3);
    for id in store.ids_in_box_sections(area) {
        let eligible = {
            let sensing = Default::default();
            let targets = TargetView::new(body, store, level, &sensing);
            store.get(id).is_some_and(|target| {
                matches!(target.data(), EntityData::GeneralLiving(data) if data.is_mob())
                    && target.bounding_box().intersects(area)
            }) && targets
                .facts(id)
                .is_some_and(|target| scary(&targets, &target))
        };
        if !eligible {
            continue;
        }
        let Some(hurt) = hurt_entity_from_body(
            body,
            store,
            level,
            id,
            "minecraft:mob_attack",
            (1 + state) as f32,
        ) else {
            return false;
        };
        if hurt {
            store.with_entity_mut(id, |target, store| {
                add_active_effect_in_level(target, store, level, poison(state))
            });
            sound(body, store, "minecraft:entity.puffer_fish.sting", false);
        }
    }
    true
}

pub(crate) fn poison(state: i32) -> ActiveMobEffect {
    ActiveMobEffect::new("minecraft:poison", 60 * state, 0).expect("vanilla poison effect")
}
