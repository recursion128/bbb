//! `AbstractHorse.mobInteract` (`AbstractHorse.java:643-668`) and the prefaces
//! `Horse` and `AbstractChestedHorse` put in front of it (`Horse.java:155-174`,
//! `AbstractChestedHorse.java:143-167`): eating, the inventory, saddle and
//! armour, and the ride, for the horse, donkey, mule and both llamas.
//!
//! Camel family equipment, feeding and riding are handled here too, and so is
//! `ZombieHorse.mobInteract` (`ZombieHorse.java:148-166`), whose preface is the
//! horse's without the golden dandelion arm, and `SkeletonHorse.mobInteract`
//! (`SkeletonHorse.java:173-175`), which passes until tamed.

use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::item_types::{VANILLA_ITEM_CHEST_ID, VANILLA_ITEM_GOLDEN_DANDELION_ID};
use bbb_protocol::packets::{ItemStackSummary, SoundSource};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::entity::animal_feeding::FeedingPlayer;
use crate::entity::general_living::{
    apply_abstract_horse_make_mad, do_player_ride_body_writes, AgeableMobRetainedState,
};
use crate::entity::level::EntityRegistryTagView;
use crate::entity::living::dimensions_growth::SizeGrowthCollisionView;
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

/// `ItemTags.HORSE_FOOD` and `ItemTags.LLAMA_FOOD`, `isFood`
/// (`AbstractHorse.java:510-512`, `Llama.java:173-175`).
const HORSE_FOOD_TAG: &str = "minecraft:horse_food";
const LLAMA_FOOD_TAG: &str = "minecraft:llama_food";
const ITEM_REGISTRY: &str = "minecraft:item";

/// What one `mobInteract` click on an equine answered.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HorseInteraction {
    /// This arm did not apply; the target's remaining `mobInteract` chain runs.
    Pass,
    /// `doPlayerRide` ran its two body writes and the click owes the caller
    /// `player.startRiding(this)` (`AbstractHorse.java:494-502`), with the
    /// rotation the rider is given first (`:498-499`).
    Mounted { y_rot: f32, x_rot: f32 },
    /// The untamed non-food arm (`Horse.java:164-167`): `makeMad()`.
    MadeMad,
    /// `fedFood` ate one item (`AbstractHorse.java:413-420`): `SUCCESS_SERVER`.
    Fed,
    /// A saddle or body armour (`:653-663`) or a chest
    /// (`AbstractChestedHorse.java:157-160`) went onto the mount: `SUCCESS`.
    Equipped(crate::entity::item_slots::SlotWriteEffects),
    /// `openCustomInventoryScreen` (`:648-651`): the caller opens the mount's
    /// inventory menu.
    OpenInventory,
    /// An arm this runtime has no owner for would have run, so nothing is
    /// written and the click changes nothing.
    Refused,
}

/// The `mobInteract` override chain of the clicked equine.
pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    secondary_use_active: bool,
    tags: &EntityRegistryTagView,
    growth: Option<&SizeGrowthCollisionView<'_>>,
) -> HorseInteraction {
    let Some(facts) = HorseFacts::of(store, target_id) else {
        return HorseInteraction::Pass;
    };
    if facts.camel {
        // `Camel.mobInteract` (`Camel.java:374-394`).
        if secondary_use_active && !facts.baby {
            return HorseInteraction::OpenInventory;
        }
        match crate::entity::item_slots::equip_on_interact(store, target_id, player, tags) {
            Ok(Some(effects)) => return HorseInteraction::Equipped(effects),
            Ok(None) => {}
            Err(()) => return HorseInteraction::Refused,
        }
        if item_in_tag(player.hand_stack, facts.food_tag(), tags) {
            return fed_food(store, target_id, player, facts, growth);
        }
        if !facts.baby && store.passengers(target_id).len() < 2 {
            return mount(store, target_id);
        }
        return HorseInteraction::Pass;
    }
    if facts.skeleton_horse {
        if !facts.tamed {
            return HorseInteraction::Pass;
        }
        return abstract_horse_mob_interact(
            store,
            target_id,
            player,
            secondary_use_active,
            facts,
            tags,
        );
    }
    let hand = &*player.hand_stack;
    let holds_golden_dandelion =
        !facts.zombie_horse && hand.item_id == Some(VANILLA_ITEM_GOLDEN_DANDELION_ID);
    let should_open_inventory = !facts.baby && facts.tamed && secondary_use_active;
    if !facts.vehicle
        && !should_open_inventory
        && !(facts.baby && holds_golden_dandelion)
        && !stack_is_empty(hand)
    {
        if item_in_tag(hand, facts.food_tag(), tags) {
            return fed_food(store, target_id, player, facts, growth);
        }
        if !facts.tamed {
            let Some(mut target) = store.detach(target_id) else {
                return HorseInteraction::Pass;
            };
            let made_mad = apply_abstract_horse_make_mad(&mut target, store);
            store.attach(target);
            return if made_mad {
                HorseInteraction::MadeMad
            } else {
                HorseInteraction::Refused
            };
        }
        if facts.chested == Some(false) && hand.item_id == Some(VANILLA_ITEM_CHEST_ID) {
            return equip_chest(store, target_id, player, facts);
        }
    }
    abstract_horse_mob_interact(store, target_id, player, secondary_use_active, facts, tags)
}

/// `AbstractHorse.mobInteract` (`AbstractHorse.java:643-668`).
fn abstract_horse_mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    secondary_use_active: bool,
    facts: HorseFacts,
    tags: &EntityRegistryTagView,
) -> HorseInteraction {
    // `:644-646`: `super.mobInteract` is `Animal.mobInteract`, which the
    // feeding seam already owns.
    if facts.vehicle || facts.baby {
        return HorseInteraction::Pass;
    }
    if facts.tamed && secondary_use_active {
        return HorseInteraction::OpenInventory;
    }
    if !stack_is_empty(player.hand_stack) {
        match equip_from_hand(store, target_id, player, tags) {
            Ok(Some(effects)) => return HorseInteraction::Equipped(effects),
            Ok(None) => {}
            Err(()) => return HorseInteraction::Refused,
        }
    }
    mount(store, target_id)
}

/// `AbstractHorse.doPlayerRide` (`AbstractHorse.java:494-502`).
fn mount(store: &mut EntityStore, target_id: i32) -> HorseInteraction {
    // `TraderLlama.doPlayerRide` (`TraderLlama.java:75-80`).
    if store.get(target_id).is_some_and(|target| {
        target.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
            && crate::entity::leash::leash_holder(target)
                .and_then(|holder| store.get(holder))
                .is_some_and(|holder| {
                    holder.entity_type_id()
                        == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID
                })
    }) {
        return HorseInteraction::Refused;
    }
    let Some(mut target) = store.detach(target_id) else {
        return HorseInteraction::Pass;
    };
    do_player_ride_body_writes(&mut target);
    let (y_rot, x_rot) = (target.y_rot(), target.x_rot());
    store.attach(target);
    HorseInteraction::Mounted { y_rot, x_rot }
}

/// `AbstractChestedHorse.equipChest` (`AbstractChestedHorse.java:169-174`)
/// and `playChestEquipsSound` (`:181-183`, `Llama.java:294-296`,
/// `Mule.java:49-51`).
fn equip_chest(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    facts: HorseFacts,
) -> HorseInteraction {
    let chest = RuntimeItemStack::from_summary(player.hand_stack.clone()).with_count(1);
    let Some(target) = store.get_mut(target_id) else {
        return HorseInteraction::Pass;
    };
    if crate::entity::horse_phenotype::set_command_chest(target, &chest) != Ok(true) {
        return HorseInteraction::Refused;
    }
    let (silent, position) = (target.is_silent(), target.position());
    let sound = if facts.llama {
        "minecraft:entity.llama.chest"
    } else if target.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MULE_ID {
        "minecraft:entity.mule.chest"
    } else {
        "minecraft:entity.donkey.chest"
    };
    if !silent {
        let random = store.random();
        let pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
        store.play_entity_sound(PendingEntitySound {
            position,
            resource_id: sound.to_owned(),
            source: SoundSource::Neutral,
            volume: 1.0,
            pitch,
        });
    }
    if !player.infinite_materials {
        player.hand_stack.count -= 1;
        if player.hand_stack.count <= 0 {
            *player.hand_stack = ItemStackSummary::empty();
        }
    }
    HorseInteraction::Equipped(Default::default())
}

/// `ItemStack.interactLivingEntity`'s `Equippable.equipOnTarget`
/// (`ItemStack.java:568-578`, `Equippable.java:159-172`) and the body-armour
/// arm (`AbstractHorse.java:660-663`).
fn equip_from_hand(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> Result<Option<crate::entity::item_slots::SlotWriteEffects>, ()> {
    if let Some(effects) =
        crate::entity::item_slots::equip_on_interact(store, target_id, player, tags)?
    {
        return Ok(Some(effects));
    }
    let stack = RuntimeItemStack::from_summary(player.hand_stack.clone());
    let target = store.get(target_id).ok_or(())?;
    let equippable = crate::entity::living::valid_equipment::equippable(&stack).ok_or(())?;
    let Some(equippable) = equippable else {
        return Ok(None);
    };
    let slot = bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id(equippable.slot_id)
        .ok_or(())?
        .slot;
    let fits = |target: &Entity| {
        crate::entity::living::valid_equipment::is_equippable_in_slot(target, &stack, slot, tags)
            .map(|fits| {
                fits && target
                    .living()
                    .is_some_and(|living| !living.has_item_in_slot(slot))
            })
    };
    if slot != EquipmentSlot::Body || !fits(target).ok_or(())? {
        return Ok(None);
    }
    let target = store.get_mut(target_id).ok_or(())?;
    let effects =
        crate::entity::item_slots::set_living_item(target, slot, Some(stack.with_count(1)))
            .map_err(|_| ())?;
    if let EntityData::GeneralLiving(data) = &mut target.data {
        if let Some(mob) = data.mob.as_mut() {
            mob.set_guaranteed_drop(slot);
            mob.set_persistence_required(true);
        }
    }
    if !player.infinite_materials {
        player.hand_stack.count -= 1;
        if player.hand_stack.count <= 0 {
            *player.hand_stack = ItemStackSummary::empty();
        }
    }
    Ok(Some(effects))
}

/// `AbstractHorse.fedFood` (`AbstractHorse.java:413-420`) over `handleEating`
/// (`:422-491`) and `Llama.handleEating` (`Llama.java:178-232`).
fn fed_food(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    facts: HorseFacts,
    growth: Option<&SizeGrowthCollisionView<'_>>,
) -> HorseInteraction {
    let Some(item) = player
        .hand_stack
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
    else {
        return HorseInteraction::Pass;
    };
    let Some(food) = (if facts.camel {
        // `Camel.handleEating` (`Camel.java:416-459`); CamelHusk.java:71-73 forbids breeding.
        Some(HorseFood::new(2.0, 10, 0, !facts.camel_husk))
    } else {
        HorseFood::of(facts.llama, item.resource_id)
    }) else {
        return HorseInteraction::Pass;
    };
    let Some(mut target) = store.detach(target_id) else {
        return HorseInteraction::Pass;
    };
    let eaten = handle_eating(&mut target, store, player, facts, food, growth);
    store.attach(target);
    let Some(eaten) = eaten else {
        return HorseInteraction::Refused;
    };
    if !eaten {
        return HorseInteraction::Pass;
    }
    crate::entity::animal_feeding::use_player_item_for_taming(player);
    if !facts.llama {
        store.queue_command(crate::entity::EntityCommand::GameEvent {
            kind: crate::level_event::VanillaGameEvent::EAT,
            position: store.get(target_id).expect("the fed equine").position(),
            source_entity_id: Some(target_id),
        });
    }
    HorseInteraction::Fed
}

/// One food's `heal`, `ageUp`, `temper` and love arm.
#[derive(Debug, Clone, Copy)]
struct HorseFood {
    heal: f32,
    age_up: i32,
    temper: i32,
    breeds: bool,
}

impl HorseFood {
    const fn new(heal: f32, age_up: i32, temper: i32, breeds: bool) -> Self {
        Self {
            heal,
            age_up,
            temper,
            breeds,
        }
    }

    fn of(llama: bool, resource_id: &str) -> Option<Self> {
        Some(match (llama, resource_id) {
            // `Llama.java:183-195`.
            (true, "minecraft:wheat") => Self::new(2.0, 10, 3, false),
            (true, "minecraft:hay_block") => Self::new(10.0, 90, 6, true),
            (true, _) => return None,
            // `AbstractHorse.java:427-465`.
            (false, "minecraft:wheat") => Self::new(2.0, 20, 3, false),
            (false, "minecraft:sugar") => Self::new(1.0, 30, 3, false),
            (false, "minecraft:hay_block") => Self::new(20.0, 180, 0, false),
            (false, "minecraft:apple" | "minecraft:carrot") => Self::new(3.0, 60, 3, false),
            (false, "minecraft:red_mushroom") => Self::new(3.0, 0, 3, false),
            (false, "minecraft:golden_carrot") => Self::new(4.0, 60, 5, true),
            (false, "minecraft:golden_apple" | "minecraft:enchanted_golden_apple") => {
                Self::new(10.0, 240, 10, true)
            }
            (false, _) => return None,
        })
    }
}

/// `handleEating`'s writes; `Some(true)` when the item was used and `None`
/// when the growth could not be resolved.
fn handle_eating(
    target: &mut Entity,
    store: &mut EntityStore,
    player: &FeedingPlayer<'_>,
    facts: HorseFacts,
    food: HorseFood,
    growth: Option<&SizeGrowthCollisionView<'_>>,
) -> Option<bool> {
    let mut used = false;
    let EntityData::GeneralLiving(data) = target.data() else {
        return Some(false);
    };
    let ageable = data.ageable_retained()?;
    if food.breeds
        && facts.tamed
        && ageable.age() == 0
        && crate::entity::animal_breeding::can_fall_in_love(target)
    {
        used = true;
        crate::entity::animal_breeding::set_in_love(target, store, Some(player.uuid));
    }
    let (health, max_health) = target
        .living()
        .map(|living| (living.health(), living.max_health()))?;
    if health < max_health && food.heal > 0.0 && health > 0.0 {
        crate::entity::living::set_health(target, health + food.heal);
        used = true;
    }
    if facts.baby && food.age_up > 0 && !ageable.age_locked() {
        if !crate::entity::animal_feeding::age_up(target, food.age_up, false, growth) {
            return None;
        }
        used = true;
    }
    let temper = data_horse_temper(target);
    let max_temper =
        crate::entity::general_living::abstract_horse_max_temper(target.entity_type_id());
    if food.temper > 0 && (used || !facts.tamed) && temper < max_temper {
        crate::entity::general_living::modify_abstract_horse_temper(target, food.temper);
        used = true;
    }
    if used && !facts.llama && !facts.camel {
        if let EntityData::GeneralLiving(data) = &mut target.data {
            if data.abstract_horse_retained_mut()?.open_mouth() {
                target.mark_synched_dirty(
                    bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID,
                );
            }
        }
    }
    if used && !target.is_silent() {
        // `eating()` (`AbstractHorse.java:258-276`).
        if let Some(sound) = eating_sound(target) {
            let random = store.random();
            let pitch = 1.0 + (random.next_float() - random.next_float()) * 0.2;
            store.play_entity_sound(PendingEntitySound {
                position: target.position(),
                resource_id: sound.to_owned(),
                source: SoundSource::Neutral,
                volume: 1.0,
                pitch,
            });
        }
    }
    Some(used)
}

fn data_horse_temper(target: &Entity) -> i32 {
    match target.data() {
        EntityData::GeneralLiving(data) => data
            .abstract_horse_retained()
            .map_or(0, |horse| horse.temper()),
        _ => 0,
    }
}

/// `getEatingSound` (`Horse.java:140-142`, `Donkey.java:35-37`,
/// `Mule.java:34-36`, `Llama.java:284-286`, `ZombieHorse.java:110-112`).
fn eating_sound(target: &Entity) -> Option<&'static str> {
    let EntityData::GeneralLiving(data) = target.data() else {
        return None;
    };
    let baby = data
        .ageable_retained()
        .is_some_and(AgeableMobRetainedState::is_baby);
    Some(match data.entity_resource_id() {
        "minecraft:camel" => "minecraft:entity.camel.eat",
        "minecraft:camel_husk" => "minecraft:entity.camel_husk.eat",
        "minecraft:horse" if baby => "minecraft:entity.baby_horse.eat",
        "minecraft:horse" => "minecraft:entity.horse.eat",
        "minecraft:donkey" => "minecraft:entity.donkey.eat",
        "minecraft:mule" => "minecraft:entity.mule.eat",
        "minecraft:zombie_horse" => "minecraft:entity.zombie_horse.eat",
        "minecraft:llama" | "minecraft:trader_llama" => "minecraft:entity.llama.eat",
        _ => return None,
    })
}

/// The reads that decide which arm runs.
#[derive(Debug, Clone, Copy)]
struct HorseFacts {
    baby: bool,
    tamed: bool,
    vehicle: bool,
    llama: bool,
    camel: bool,
    camel_husk: bool,
    zombie_horse: bool,
    skeleton_horse: bool,
    /// `AbstractChestedHorse.hasChest`, `None` for a horse.
    chested: Option<bool>,
}

impl HorseFacts {
    fn of(store: &EntityStore, target_id: i32) -> Option<Self> {
        let target = store.get(target_id)?;
        let EntityData::GeneralLiving(data) = target.data() else {
            return None;
        };
        let llama = match data.entity_resource_id() {
            "minecraft:horse"
            | "minecraft:donkey"
            | "minecraft:mule"
            | "minecraft:zombie_horse"
            | "minecraft:skeleton_horse"
            | "minecraft:camel"
            | "minecraft:camel_husk" => false,
            "minecraft:llama" | "minecraft:trader_llama" => true,
            _ => return None,
        };
        let horse = data.abstract_horse_retained()?;
        let camel_husk = data.entity_resource_id() == "minecraft:camel_husk";
        let camel = camel_husk || data.entity_resource_id() == "minecraft:camel";
        let zombie_horse = data.entity_resource_id() == "minecraft:zombie_horse";
        let skeleton_horse = data.entity_resource_id() == "minecraft:skeleton_horse";
        let chested = if camel || zombie_horse || skeleton_horse {
            None
        } else {
            match data.horse_phenotype()? {
                crate::entity::horse_phenotype::HorsePhenotypeState::Horse { .. } => None,
                crate::entity::horse_phenotype::HorsePhenotypeState::Chested { has_chest }
                | crate::entity::horse_phenotype::HorsePhenotypeState::Llama {
                    has_chest, ..
                } => Some(has_chest),
            }
        };
        Some(Self {
            // `CamelHusk.isBaby` always returns false (`CamelHusk.java:76-78`).
            baby: !camel_husk
                && data
                    .ageable_retained()
                    .is_some_and(AgeableMobRetainedState::is_baby),
            // `Camel.isTamed` always returns true (`Camel.java:645-647`).
            tamed: camel || horse.is_tame(),
            // `Entity.isVehicle` (`Entity.java:2595-2597`).
            vehicle: !store.passengers(target_id).is_empty(),
            llama,
            camel,
            camel_husk,
            zombie_horse,
            skeleton_horse,
            chested,
        })
    }

    const fn food_tag(self) -> &'static str {
        if self.camel_husk {
            "minecraft:camel_husk_food"
        } else if self.camel {
            "minecraft:camel_food"
        } else if self.zombie_horse {
            "minecraft:zombie_horse_food"
        } else if self.llama {
            LLAMA_FOOD_TAG
        } else {
            HORSE_FOOD_TAG
        }
    }
}

fn stack_is_empty(stack: &ItemStackSummary) -> bool {
    stack.item_id.is_none() || stack.count <= 0
}

fn item_in_tag(stack: &ItemStackSummary, tag_id: &str, tags: &EntityRegistryTagView) -> bool {
    let Some(item) = stack
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
    else {
        return false;
    };
    tags.contains(ITEM_REGISTRY, tag_id, item.resource_id)
}

#[cfg(test)]
mod tests;
