//! Food, equipment and ordinary riding for nautiluses.

use bbb_protocol::packets::{EntityEvent, ItemStackSummary};

use crate::connection::ClientboundPlayPacket;
use crate::entity::animal_feeding::FeedingPlayer;
use crate::entity::leash::LeashInteraction;
use crate::entity::living::dimensions_growth::SizeGrowthCollisionView;
use crate::entity::store::PendingEntitySound;
use crate::entity::{EntityData, EntityRegistryTagView, EntityStore};

// AbstractNautilus.java:397-443.
pub(super) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    secondary_use: bool,
    tags: &EntityRegistryTagView,
    growth: Option<&SizeGrowthCollisionView<'_>>,
    mob_item_used: &mut bool,
) -> LeashInteraction {
    let Some(target) = store.get(target_id) else {
        return LeashInteraction::Pass;
    };
    let EntityData::GeneralLiving(data) = target.data() else {
        return LeashInteraction::Pass;
    };
    let zombie = match data.entity_resource_id() {
        "minecraft:nautilus" => false,
        "minecraft:zombie_nautilus" => true,
        _ => return LeashInteraction::Pass,
    };
    let baby = data.is_baby();
    let growth_age = data
        .ageable_retained()
        .filter(|age| age.can_age_up())
        .map(|age| age.age());
    let tame = data.is_tame();
    let hurt = target
        .living()
        .is_some_and(|living| living.health() < living.max_health());
    let persist = |store: &mut EntityStore| {
        if let Some(target) = store.get_mut(target_id) {
            crate::entity::conversion::set_persistence_required(target);
        }
    };
    if baby {
        // AbstractNautilus.java:399-400; Animal.java:150-155.
        let Some(age) =
            growth_age.filter(|_| in_tag(player.hand_stack, "minecraft:nautilus_food", tags))
        else {
            return LeashInteraction::Pass;
        };
        let mut target = store.detach(target_id).expect("nautilus");
        let grew = crate::entity::animal_feeding::age_up(
            &mut target,
            crate::entity::animal_feeding::speed_up_seconds_when_feeding(age.wrapping_neg()),
            true,
            growth,
        );
        store.attach(target);
        if !grew {
            return LeashInteraction::Refused;
        }
        persist(store);
        let remainder = if in_tag(player.hand_stack, "minecraft:nautilus_bucket_food", tags) {
            crate::entity::animal_feeding::use_player_bucket_food(player)
        } else {
            crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used)
        };
        play_eating_sound(store, target_id, zombie);
        return match remainder {
            Some(item) => LeashInteraction::FedWithRemainder {
                server_swing: false,
                item,
            },
            None => LeashInteraction::GrewUp,
        };
    }
    if tame && secondary_use {
        persist(store);
        return LeashInteraction::OpenMountInventory {
            vehicle_entity_id: target_id,
        };
    }
    // AbstractNautilus.java:98-108.
    let food = in_tag(
        player.hand_stack,
        if tame {
            "minecraft:nautilus_food"
        } else {
            "minecraft:nautilus_taming_items"
        },
        tags,
    );
    if food && (!tame || hurt) {
        let nutrition = if tame {
            match crate::entity::taming::food_nutrition(player.hand_stack) {
                Ok(value) => value,
                Err(()) => return LeashInteraction::Refused,
            }
        } else {
            None
        };
        persist(store);
        let remainder = if in_tag(player.hand_stack, "minecraft:nautilus_bucket_food", tags) {
            crate::entity::animal_feeding::use_player_bucket_food(player)
        } else {
            crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used)
        };
        let outcome = if !tame {
            let success = store.random().next_int(3) == 0;
            if success {
                let mut target = store.detach(target_id).expect("nautilus");
                crate::entity::taming::tame_player(&mut target, store, player.uuid);
                store.attach(target);
                store.stop_mob_navigation(target_id);
            }
            store.broadcast_to_tracking(
                target_id,
                ClientboundPlayPacket::EntityEvent(EntityEvent {
                    entity_id: target_id,
                    event_id: if success { 7 } else { 6 },
                }),
            );
            if success {
                LeashInteraction::Tamed {
                    server_swing: true,
                    remainder,
                }
            } else {
                LeashInteraction::TamingFailed {
                    server_swing: true,
                    remainder,
                }
            }
        } else {
            // TamableAnimal.java:135-140; AbstractNautilus.java:418-420.
            let target = store.get_mut(target_id).expect("nautilus");
            let health = target.living().expect("living nautilus").health();
            if health > 0.0 {
                crate::entity::living::set_health(
                    target,
                    health + nutrition.map_or(1.0, |value| value as f32 * 2.0),
                );
            }
            match remainder {
                Some(item) => LeashInteraction::FedWithRemainder {
                    server_swing: false,
                    item,
                },
                None => LeashInteraction::AnimalFed,
            }
        };
        play_eating_sound(store, target_id, zombie);
        return outcome;
    }
    if player.hand_stack.item_id.is_some() && player.hand_stack.count > 0 {
        match crate::entity::item_slots::equip_on_interact(store, target_id, player, tags) {
            Ok(Some(effects)) => {
                persist(store);
                return LeashInteraction::MountEquipped {
                    entity_id: target_id,
                    effects,
                };
            }
            Err(()) => return LeashInteraction::Refused,
            Ok(None) => {}
        }
    }
    // Animal.java:139-147; both nautilus types inherit ordinary courtship.
    if food
        && store.get(target_id).is_some_and(|target| {
            matches!(target.data(), EntityData::GeneralLiving(data)
                if data.ageable_retained().is_some_and(|age| age.age() == 0))
                && crate::entity::animal_breeding::can_fall_in_love(target)
        })
    {
        let mut target = store.detach(target_id).expect("nautilus");
        let loved =
            crate::entity::animal_breeding::set_in_love(&mut target, store, Some(player.uuid));
        store.attach(target);
        if !loved {
            return LeashInteraction::Refused;
        }
        persist(store);
        let remainder = if in_tag(player.hand_stack, "minecraft:nautilus_bucket_food", tags) {
            crate::entity::animal_feeding::use_player_bucket_food(player)
        } else {
            crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used)
        };
        play_eating_sound(store, target_id, zombie);
        return match remainder {
            Some(item) => LeashInteraction::FedWithRemainder {
                server_swing: true,
                item,
            },
            None => LeashInteraction::FellInLove,
        };
    }
    if tame && !secondary_use && !food {
        persist(store);
        LeashInteraction::Mounted {
            vehicle_entity_id: target_id,
            rotation: None,
        }
    } else {
        LeashInteraction::Pass
    }
}

fn in_tag(stack: &ItemStackSummary, tag: &str, tags: &EntityRegistryTagView) -> bool {
    stack
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
        .is_some_and(|item| {
            stack.count > 0 && tags.contains("minecraft:item", tag, item.resource_id)
        })
}

// Nautilus.java:101-103; SoundEvents.java:136.
fn play_eating_sound(store: &mut EntityStore, target_id: i32, zombie: bool) {
    let Some(target) = store.get(target_id).filter(|target| !target.is_silent()) else {
        return;
    };
    let position = target.position();
    let source = crate::entity::living::living_sound_source(target);
    let baby = matches!(target.data(), EntityData::GeneralLiving(data) if data.is_baby());
    let pitch = crate::entity::living::voice_pitch(store, baby);
    store.play_entity_sound(PendingEntitySound {
        position,
        resource_id: if zombie {
            "minecraft:entity.zombie_nautilus.eat"
        } else if baby {
            "minecraft:entity.baby_nautilus.eat"
        } else {
            "minecraft:entity.nautilus.eat"
        }
        .to_owned(),
        source,
        volume: 1.0,
        pitch,
    });
}
