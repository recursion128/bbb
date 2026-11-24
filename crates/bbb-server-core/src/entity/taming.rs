//! `Wolf.mobInteract` (`Wolf.java:445-498`), `Cat.mobInteract`
//! (`Cat.java:396-444`) and `Parrot.mobInteract` (`Parrot.java:252-299`), the
//! overrides in this runtime that own a taming roll and a sit toggle.
//!
//! Both split on `if (this.isTame())` (`Wolf.java:447`, `Cat.java:398`). The
//! untamed side is the taming roll itself (`Wolf.java:491-495`,
//! `Cat.java:427-436`) over `TamableAnimal.tame`
//! (`TamableAnimal.java:168-174`). The tamed side runs the collar dye and the
//! healing feed before `super.mobInteract`, and the sit toggle after it
//! (`Wolf.java:473-479`, `Cat.java:419-423`) -- which is why this module is
//! asked twice per click, once on each side of the feeding branch.

pub(super) mod death_message;

use crate::connection::ClientboundPlayPacket;
use bbb_protocol::entity_synched_data::{
    VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID, VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID,
};
use bbb_protocol::item_types::VANILLA_ITEM_BONE_ID;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::packets::{EntityEvent, SoundSource};
use uuid::Uuid;

use crate::entity::animal_feeding::FeedingPlayer;
use crate::entity::level::EntityRegistryTagView;
use crate::entity::store::{EntityStore, PendingTamedAnimal};
use crate::entity::{Entity, EntityData};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID;

/// `#minecraft:cat_food` (`Cat.java:447-449`) and `#minecraft:wolf_food`
/// (`Wolf.java:538-540`), the two `isFood` answers.
const CAT_FOOD_TAG: &str = "minecraft:cat_food";
const WOLF_FOOD_TAG: &str = "minecraft:wolf_food";

/// `ItemTags.CAT_COLLAR_DYES` (`Cat.java:400`) and `ItemTags.WOLF_COLLAR_DYES`
/// (`Wolf.java:453`).
const CAT_COLLAR_DYES_TAG: &str = "minecraft:cat_collar_dyes";
const WOLF_COLLAR_DYES_TAG: &str = "minecraft:wolf_collar_dyes";
const ITEM_REGISTRY: &str = "minecraft:item";

/// `ItemTags.PARROT_FOOD` and `PARROT_POISONOUS_FOOD` (`Parrot.java:254`,
/// `:280`) and `SoundEvents.PARROT_EAT`.
const PARROT_FOOD_TAG: &str = "minecraft:parrot_food";
const PARROT_POISONOUS_FOOD_TAG: &str = "minecraft:parrot_poisonous_food";
const PARROT_EAT_SOUND: &str = "minecraft:entity.parrot.eat";

/// `TamableAnimal.handleEntityEvent`'s two taming bytes
/// (`TamableAnimal.java:105-113`): hearts for a taming that took, smoke for one
/// that did not.
const TAMING_SUCCEEDED_ENTITY_EVENT: i8 = 7;
const TAMING_FAILED_ENTITY_EVENT: i8 = 6;

/// `Wolf.applyTamingSideEffects` (`Wolf.java:425-432`).
const WOLF_TAME_MAX_HEALTH: f64 = 40.0;

/// What one taming `mobInteract` arm answered.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TamingInteraction {
    /// The arm did not apply; the target's remaining `mobInteract` body runs.
    Pass,
    /// `Wolf.mobInteract`'s `SUCCESS_SERVER` (`Wolf.java:494`) or
    /// `Cat.mobInteract`'s `SUCCESS` (`Cat.java:435`): one food item was spent
    /// and the roll took, so the animal now has an owner.
    Tamed,
    /// The same two results with the roll refused: the item is still spent and
    /// the click is still consumed, and only the smoke shows.
    TamingFailed,
    /// `Wolf.java:489` and `Cat.java:409`: the collar took a new dye and one
    /// dye item was spent.
    CollarDyed,
    /// `Wolf.java:450` and `Cat.java:416` over `TamableAnimal.feed`
    /// (`TamableAnimal.java:135-140`): a hurt tame animal ate one food item
    /// and healed.
    Fed,
    /// `Wolf.java:475-479` and `Cat.java:421-422`: the click that
    /// `super.mobInteract` did not consume toggled the sit order instead.
    SitToggled,
    /// The parrot ate poisonous food; the world applies its poison and hurt.
    PoisonousFood,
    /// `Wolf.java:454-458`: the owner put body armour on the wolf.
    ArmourEquipped(crate::entity::item_slots::SlotWriteEffects),
    /// `Wolf.java:460-471`: one repair item mended the sitting wolf's armour.
    ArmourRepaired,
    /// A read this arm needs has no answer here, so nothing is written.
    Refused,
}

/// Everything `Wolf.mobInteract` and `Cat.mobInteract` do before
/// `super.mobInteract`, which `Mob.interact` reaches before
/// `Animal.mobInteract`.
pub(crate) fn mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    mob_item_used: &mut bool,
) -> TamingInteraction {
    let Some(target) = store.get(target_id) else {
        return TamingInteraction::Pass;
    };
    let EntityData::GeneralLiving(data) = target.data() else {
        return TamingInteraction::Pass;
    };
    if data.entity_resource_id() == "minecraft:parrot" && data.tamable_retained().is_some() {
        return parrot_mob_interact(store, target_id, player, tags, mob_item_used);
    }
    let Some(kind) = TamingKind::of(data.entity_resource_id()) else {
        return TamingInteraction::Pass;
    };
    if data.tamable_retained().is_none() {
        return TamingInteraction::Pass;
    }
    // `if (this.isTame())` (`Wolf.java:447`, `Cat.java:398`).
    if data.is_tame() {
        tamed_arms(store, target_id, player, tags, kind, mob_item_used)
    } else {
        untamed_arms(store, target_id, player, tags, kind, mob_item_used)
    }
}

/// The `else` of `if (this.isTame())`: the taming roll itself.
fn untamed_arms(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    kind: TamingKind,
    mob_item_used: &mut bool,
) -> TamingInteraction {
    let Some(target) = store.get(target_id) else {
        return TamingInteraction::Pass;
    };
    match kind {
        // `itemStack.is(Items.BONE) && !this.isAngry()` (`Wolf.java:491`).
        TamingKind::Wolf => {
            let angry = crate::entity::ai::mob::target::anger::read(target)
                .is_some_and(|anger| anger.is_angry(store.game_time()));
            if player.hand_stack.item_id != Some(VANILLA_ITEM_BONE_ID) || angry {
                return TamingInteraction::Pass;
            }
        }
        // `this.isFood(itemStack)` (`Cat.java:427`) over `#minecraft:cat_food`.
        TamingKind::Cat => {
            let Some(item) = player
                .hand_stack
                .item_id
                .and_then(bbb_protocol::item_types::vanilla_item_type)
            else {
                return TamingInteraction::Pass;
            };
            if !tags.contains(ITEM_REGISTRY, CAT_FOOD_TAG, item.resource_id) {
                return TamingInteraction::Pass;
            }
        }
    }
    // `Cat.playEatingSound` (`Cat.java:284-286`) is the one read either arm
    // needs beyond the roll, and a cat whose sound variant this build cannot
    // name has no sound to play, so the click is refused rather than half run.
    let eat_sound = match kind {
        TamingKind::Wolf => None,
        TamingKind::Cat => {
            match super::cat_sounds::resolve(store, target, super::cat_sounds::CatSound::Eat) {
                Some(sound) => Some(sound),
                None => return TamingInteraction::Refused,
            }
        }
    };
    // `itemStack.consume(1, player)` (`Wolf.java:492`) and
    // `usePlayerItem` (`Cat.java:429`), both before the roll.
    match kind {
        TamingKind::Wolf => crate::entity::animal_feeding::use_player_item_for_taming(player),
        TamingKind::Cat => {
            crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used);
        }
    }
    // `this.random.nextInt(3) == 0` (`Wolf.java:501`, `Cat.java:475`): one draw
    // off the body's own random, taken whether or not it takes.
    let tamed = store.random().next_int(3) == 0;
    if tamed {
        let Some(mut target) = store.detach(target_id) else {
            return TamingInteraction::Pass;
        };
        tame(&mut target, store, player.uuid, Some(kind));
        store.attach(target);
        // `navigation.stop()` and `setTarget(null)` (`Wolf.java:503-504`);
        // `Cat.tryToTame` makes neither call.
        if matches!(kind, TamingKind::Wolf) {
            store.stop_mob_navigation(target_id);
            crate::entity::ai::mob::target::sense::clear_mob_target(store, target_id);
        }
    }
    if let (TamingKind::Cat, Some(sound)) = (kind, eat_sound) {
        // `this.setPersistenceRequired()` (`Cat.java:431`), which a cat takes
        // whether or not the roll took.
        if let Some(target) = store.get_mut(target_id) {
            crate::entity::conversion::set_persistence_required(target);
        }
        // `this.playEatingSound()` (`:432`) through
        // `this.playSound(sound, 1.0F, 1.0F)` (`:285`), which is
        // `Entity.playSound`'s coordinate form (`Entity.java:1404-1408`).
        if let Some(position) = store.get(target_id).map(Entity::position) {
            super::sound_variants::play(store, sound, position, SoundSource::Neutral, 1.0, 1.0);
        }
    }
    store.broadcast_to_tracking(
        target_id,
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id: target_id,
            event_id: if tamed {
                TAMING_SUCCEEDED_ENTITY_EVENT
            } else {
                TAMING_FAILED_ENTITY_EVENT
            },
        }),
    );
    if tamed {
        TamingInteraction::Tamed
    } else {
        TamingInteraction::TamingFailed
    }
}

/// `Parrot.mobInteract` (`Parrot.java:252-299`).
fn parrot_mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    mob_item_used: &mut bool,
) -> TamingInteraction {
    let Some(target) = store.get(target_id) else {
        return TamingInteraction::Pass;
    };
    let is_tame = matches!(target.data(), EntityData::GeneralLiving(data) if data.is_tame());
    if !is_tame && item_in_tag(player.hand_stack, PARROT_FOOD_TAG, tags) {
        let (silent, position) = (target.is_silent(), target.position());
        let _ = crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used);
        let random = store.random();
        let pitch = 1.0 + (random.next_float() - random.next_float()) * 0.2;
        if !silent {
            store.play_entity_sound(crate::entity::store::PendingEntitySound {
                position,
                resource_id: PARROT_EAT_SOUND.to_owned(),
                source: SoundSource::Neutral,
                volume: 1.0,
                pitch,
            });
        }
        // `this.random.nextInt(10) == 0` (`:271`).
        let tamed = store.random().next_int(10) == 0;
        if tamed {
            let Some(mut target) = store.detach(target_id) else {
                return TamingInteraction::Pass;
            };
            tame(&mut target, store, player.uuid, None);
            store.attach(target);
        }
        store.broadcast_to_tracking(
            target_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id: target_id,
                event_id: if tamed {
                    TAMING_SUCCEEDED_ENTITY_EVENT
                } else {
                    TAMING_FAILED_ENTITY_EVENT
                },
            }),
        );
        return if tamed {
            TamingInteraction::Tamed
        } else {
            TamingInteraction::TamingFailed
        };
    }
    if item_in_tag(player.hand_stack, PARROT_POISONOUS_FOOD_TAG, tags) {
        let _ = crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used);
        return TamingInteraction::PoisonousFood;
    }
    // `!this.isFlying() && this.isTame() && this.isOwnedBy(player)` (`:281`).
    if is_tame && target.on_ground() && is_owned_by(target, player.uuid) {
        return toggle_sit(store, target_id);
    }
    TamingInteraction::Pass
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TamingKind {
    Wolf,
    Cat,
}

impl TamingKind {
    fn of(entity_resource_id: &str) -> Option<Self> {
        match entity_resource_id {
            "minecraft:wolf" => Some(Self::Wolf),
            "minecraft:cat" => Some(Self::Cat),
            _ => None,
        }
    }

    /// `Wolf.isFood` (`Wolf.java:538-540`) and `Cat.isFood`
    /// (`Cat.java:447-449`).
    const fn food_tag(self) -> &'static str {
        match self {
            Self::Wolf => WOLF_FOOD_TAG,
            Self::Cat => CAT_FOOD_TAG,
        }
    }

    const fn collar_dyes_tag(self) -> &'static str {
        match self {
            Self::Wolf => WOLF_COLLAR_DYES_TAG,
            Self::Cat => CAT_COLLAR_DYES_TAG,
        }
    }

    /// `this.feed(player, hand, itemStack, healingFactor, defaultHeal)`
    /// (`Wolf.java:449`, `Cat.java:413`).
    const fn heal_factors(self) -> (f32, f32) {
        match self {
            Self::Wolf => (2.0, 2.0),
            Self::Cat => (1.0, 1.0),
        }
    }
}

/// `TamableAnimal.tame` (`TamableAnimal.java:168-174`) plus the sit order
/// `tryToTame` adds (`Wolf.java:505`, `Cat.java:476`); `None` is the parrot,
/// which takes neither.
fn tame(entity: &mut Entity, store: &mut EntityStore, player_uuid: Uuid, kind: Option<TamingKind>) {
    tame_player(entity, store, player_uuid);
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(tamable) = data.tamable_retained_mut().filter(|_| kind.is_some()) {
            tamable.set_ordered_to_sit(true);
        }
    }
    if kind == Some(TamingKind::Wolf) {
        apply_wolf_taming_side_effects(entity);
    }
}

pub(super) fn tame_player(entity: &mut Entity, store: &mut EntityStore, player_uuid: Uuid) {
    let entity_id = entity.id();
    let entity_type_id = entity.entity_type_id();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(tamable) = data.tamable_retained_mut() else {
        return;
    };
    // `this.setTame(true, true)` (`TamableAnimal.java:169`).
    tamable.set_tame(true);
    // `this.setOwner(player)` (`:170`).
    tamable.set_owner(Some(player_uuid));
    entity.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID);
    entity.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID);
    // `CriteriaTriggers.TAME_ANIMAL.trigger(serverPlayer, this)`
    // (`TamableAnimal.java:171-173`), queued for whoever owns the player list.
    store.queue_command(crate::entity::EntityCommand::TamedAnimal(
        PendingTamedAnimal {
            player_uuid,
            entity_id,
            entity_type_id,
        },
    ));
}

/// `baby.setOwnerReference(..)` and `baby.setTame(true, true)`
/// (`Cat.java:365-366`, `Wolf.java:590-591`, `Nautilus.java:44-45`): the owner and the flag with the
/// subtype's side effects, and none of the writes `tryToTame` adds around them
/// -- no sit order and no advancement trigger.
pub(crate) fn set_tame_offspring(entity: &mut Entity, owner: Option<Uuid>) -> bool {
    let kind = TamingKind::of(entity.data().entity_resource_id());
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(tamable) = data.tamable_retained_mut() else {
        return false;
    };
    tamable.set_owner(owner);
    tamable.set_tame(true);
    entity.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID);
    entity.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_OWNER_DATA_ID);
    if matches!(kind, Some(TamingKind::Wolf)) {
        apply_wolf_taming_side_effects(entity);
    }
    true
}

/// `Wolf.applyTamingSideEffects` (`Wolf.java:425-432`): the tame branch raises
/// the max-health base value and fills the bar, both of which the client reads
/// off the synched attribute and health rows.
fn apply_wolf_taming_side_effects(entity: &mut Entity) {
    let Some(living) = entity.living_mut() else {
        return;
    };
    let Some(max_health) = living
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
    else {
        return;
    };
    max_health.set_base_value(WOLF_TAME_MAX_HEALTH);
    crate::entity::living::set_health(entity, WOLF_TAME_MAX_HEALTH as f32);
}

/// The tame side of both overrides that runs before `super.mobInteract`:
/// `Wolf.java:448-490` and `Cat.java:399-417`.
fn tamed_arms(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    kind: TamingKind,
    mob_item_used: &mut bool,
) -> TamingInteraction {
    let Some(target) = store.get(target_id) else {
        return TamingInteraction::Pass;
    };
    let owned = is_owned_by(target, player.uuid);
    let is_food = item_in_tag(player.hand_stack, kind.food_tag(), tags);
    let Some(hurt) = below_max_health(target) else {
        return TamingInteraction::Refused;
    };
    let collar_dye = item_in_tag(player.hand_stack, kind.collar_dyes_tag(), tags);
    match kind {
        TamingKind::Wolf => {
            // `:448-451`, which precedes both the ownership test and the dye.
            if is_food && hurt {
                return feed(store, target_id, player, kind, mob_item_used);
            }
            // `:453`: only an owner holding a collar dye leaves the block that
            // holds the armour arms, `super` and the sit toggle.
            if !collar_dye || !owned {
                if owned {
                    if let Some(answer) = wolf_armour_arms(store, target_id, player, tags) {
                        return answer;
                    }
                }
                return TamingInteraction::Pass;
            }
            dye_collar(store, target_id, player, kind)
        }
        TamingKind::Cat => {
            // `:399`: a cat that is not this player's ignores the whole block.
            if !owned {
                return TamingInteraction::Pass;
            }
            // `:400-410`; a dye whose colour the collar already carries falls
            // through to `super` rather than to the food arm.
            if collar_dye {
                return dye_collar(store, target_id, player, kind);
            }
            // `:411-416`.
            if is_food && hurt {
                return feed(store, target_id, player, kind, mob_item_used);
            }
            TamingInteraction::Pass
        }
    }
}

/// The tame side of both overrides that runs after `super.mobInteract`: the
/// sit toggle (`Wolf.java:473-479`, `Cat.java:419-423`) and the cat's
/// persistence tail (`Cat.java:438-441`).
///
/// `consumed` is what `super.mobInteract` answered, which in this runtime is
/// the feeding branch.
pub(crate) fn after_super_mob_interact(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
    consumed: bool,
) -> TamingInteraction {
    let Some(target) = store.get(target_id) else {
        return TamingInteraction::Pass;
    };
    let EntityData::GeneralLiving(data) = target.data() else {
        return TamingInteraction::Pass;
    };
    let Some(kind) = TamingKind::of(data.entity_resource_id()) else {
        return TamingInteraction::Pass;
    };
    let tame = data.tamable_retained().is_some() && data.is_tame();
    let owned = is_owned_by(target, player.uuid);
    match kind {
        TamingKind::Cat => {
            // `:399-425`: an owned tame cat returns inside the block, so the
            // outer persistence tail is the one every other cat takes.
            if tame && owned {
                if consumed {
                    return TamingInteraction::Pass;
                }
                return toggle_sit(store, target_id);
            }
            if consumed {
                if let Some(target) = store.get_mut(target_id) {
                    crate::entity::conversion::set_persistence_required(target);
                }
            }
            TamingInteraction::Pass
        }
        TamingKind::Wolf => {
            if !tame {
                return TamingInteraction::Pass;
            }
            let collar_dye = item_in_tag(player.hand_stack, WOLF_COLLAR_DYES_TAG, tags);
            // `:453`: an owner's collar dye never reaches the sit toggle.
            if collar_dye && owned {
                return TamingInteraction::Pass;
            }
            // `:474`.
            if consumed || !owned {
                return TamingInteraction::Pass;
            }
            // `:475-479`.
            let toggled = toggle_sit(store, target_id);
            if toggled == TamingInteraction::SitToggled {
                if let Some(living) = store.get_mut(target_id).and_then(Entity::living_mut) {
                    living.jumping = false;
                }
                store.stop_mob_navigation(target_id);
                crate::entity::ai::mob::target::sense::clear_mob_target(store, target_id);
            }
            toggled
        }
    }
}

/// `TamableAnimal.isOwnedBy` (`TamableAnimal.java:181-183`) as the stored
/// owner reference carries it.
fn is_owned_by(entity: &Entity, player_uuid: Uuid) -> bool {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return false;
    };
    data.tamable_retained()
        .and_then(crate::entity::general_living::TamableRetainedState::owner)
        == Some(player_uuid)
}

/// `this.getHealth() < this.getMaxHealth()` (`Wolf.java:448`, `Cat.java:411`).
fn below_max_health(entity: &Entity) -> Option<bool> {
    let living = entity.living()?;
    Some(living.health() < living.max_health())
}

/// `itemStack.is(tag)` against the level's own generation; `None` is a
/// generation that cannot answer.
fn item_in_tag(stack: &ItemStackSummary, tag_id: &str, tags: &EntityRegistryTagView) -> bool {
    let Some(item) = stack
        .item_id
        .and_then(bbb_protocol::item_types::vanilla_item_type)
    else {
        return false;
    };
    tags.contains(ITEM_REGISTRY, tag_id, item.resource_id)
}

/// The two body-armour arms (`Wolf.java:454-471`) for the wolf's owner;
/// `None` falls through to `super.mobInteract` and the sit toggle.
fn wolf_armour_arms(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    tags: &EntityRegistryTagView,
) -> Option<TamingInteraction> {
    use crate::entity::general_living::TamableRetainedState;
    use bbb_protocol::packets::EquipmentSlot;
    use bbb_server_types::item_stack::RuntimeItemStack;
    let target = store.get(target_id)?;
    let EntityData::GeneralLiving(data) = target.data() else {
        return None;
    };
    let worn = target
        .living()?
        .item_by_slot(EquipmentSlot::Body)
        .filter(|armour| armour.item_id().is_some())
        .cloned();
    let Some(armour) = worn else {
        let stack = RuntimeItemStack::from_summary(player.hand_stack.clone());
        let Some(fits) = crate::entity::living::valid_equipment::is_equippable_in_slot(
            target,
            &stack,
            EquipmentSlot::Body,
            tags,
        ) else {
            return Some(TamingInteraction::Refused);
        };
        if !fits || data.ageable_retained().is_some_and(|age| age.is_baby()) {
            return None;
        }
        // `Mob.setBodyArmorItem` over `setItemSlotAndDropWhenKilled`.
        let target = store.get_mut(target_id)?;
        let Ok(effects) = crate::entity::item_slots::set_living_item(
            target,
            EquipmentSlot::Body,
            Some(stack.with_count(1)),
        ) else {
            return Some(TamingInteraction::Refused);
        };
        if let EntityData::GeneralLiving(data) = &mut target.data {
            if let Some(mob) = data.mob.as_mut() {
                mob.set_guaranteed_drop(EquipmentSlot::Body);
                mob.set_persistence_required(true);
            }
        }
        crate::entity::animal_feeding::use_player_item_for_taming(player);
        return Some(TamingInteraction::ArmourEquipped(effects));
    };
    if !data
        .tamable_retained()
        .is_some_and(TamableRetainedState::is_in_sitting_pose)
    {
        return None;
    }
    let mut repaired = armour.summary().clone();
    let (damage, max_damage) = crate::item_durability::damage_and_max(&repaired)?;
    if damage <= 0 || !is_valid_repair_item(&armour, player.hand_stack, tags)? {
        return None;
    }
    crate::entity::animal_feeding::use_player_item_for_taming(player);
    crate::item_durability::repair_to(
        &mut repaired,
        (damage - (max_damage as f32 * 0.125) as i32).max(0),
    );
    let target = store.get_mut(target_id)?;
    let position = target.position();
    let silent = target.is_silent();
    let source = crate::entity::living::living_sound_source(target);
    target.living_mut()?.update_item_slot(
        EquipmentSlot::Body,
        RuntimeItemStack::from_summary(repaired),
    )?;
    if !silent {
        store.play_entity_sound(crate::entity::PendingEntitySound {
            position,
            resource_id: "minecraft:item.wolf_armor.repair".to_owned(),
            source,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    Some(TamingInteraction::ArmourRepaired)
}

/// `ItemStack.isValidRepairItem` (`ItemStack.java:1117-1120`); `None` is a
/// `REPAIRABLE` patch this build cannot read.
fn is_valid_repair_item(
    armour: &bbb_server_types::item_stack::RuntimeItemStack,
    repair: &ItemStackSummary,
    tags: &EntityRegistryTagView,
) -> Option<bool> {
    let Some(repair_id) = repair.item_id else {
        return Some(false);
    };
    let Some(repairable) = armour
        .effective_component(
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID,
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .ok()?
    else {
        return Some(false);
    };
    if repairable.repairable_item_registry_ids.contains(&repair_id) {
        return Some(true);
    }
    let item = bbb_protocol::item_types::vanilla_item_type(repair_id)?;
    Some(
        repairable
            .repairable_item_tag
            .as_deref()
            .is_some_and(|tag| {
                tags.contains(ITEM_REGISTRY, tag.trim_start_matches('#'), item.resource_id)
            }),
    )
}

/// `Cat.setCollarColor`'s arm (`Cat.java:400-410`) and `Wolf`'s
/// (`Wolf.java:485-490`): the same three writes, with only the cat's
/// `setPersistenceRequired` differing.
fn dye_collar(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    kind: TamingKind,
) -> TamingInteraction {
    // `itemStack.get(DataComponents.DYE)` (`Wolf.java:485`, `Cat.java:401`).
    let Ok(color) = crate::entity::animal_breeding::dye_mix::stack_dye_color(player.hand_stack)
    else {
        return TamingInteraction::Refused;
    };
    // `color != null && color != this.getCollarColor()` (`:486`, `:402`).
    let Some(color) = color else {
        return TamingInteraction::Pass;
    };
    let Ok(color) = i8::try_from(color) else {
        return TamingInteraction::Refused;
    };
    let Some(target) = store.get_mut(target_id) else {
        return TamingInteraction::Pass;
    };
    if crate::entity::collar::color_of(target) == Some(color) {
        return TamingInteraction::Pass;
    }
    if !crate::entity::collar::set_collar_color(target, color) {
        return TamingInteraction::Refused;
    }
    if matches!(kind, TamingKind::Cat) {
        // `this.setPersistenceRequired()` (`Cat.java:406`).
        crate::entity::conversion::set_persistence_required(target);
    }
    // `itemStack.consume(1, player)` (`Wolf.java:488`, `Cat.java:405`).
    crate::entity::animal_feeding::use_player_item_for_taming(player);
    TamingInteraction::CollarDyed
}

/// `TamableAnimal.feed` (`TamableAnimal.java:135-140`).
fn feed(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut FeedingPlayer<'_>,
    kind: TamingKind,
    mob_item_used: &mut bool,
) -> TamingInteraction {
    // `itemStack.get(DataComponents.FOOD)` (`:136`), read before the stack is
    // spent.
    let Ok(nutrition) = food_nutrition(player.hand_stack) else {
        return TamingInteraction::Refused;
    };
    let eat_sound = match kind {
        TamingKind::Wolf => None,
        // `Cat.playEatingSound` (`Cat.java:284-286`) is the one read this arm
        // needs beyond the heal.
        TamingKind::Cat => match store.get(target_id).and_then(|target| {
            super::cat_sounds::resolve(store, target, super::cat_sounds::CatSound::Eat)
        }) {
            Some(sound) => Some(sound),
            None => return TamingInteraction::Refused,
        },
    };
    let (healing_factor, default_heal) = kind.heal_factors();
    // `this.usePlayerItem(player, hand, itemStack)` (`:137`).
    let _ = crate::entity::animal_feeding::use_mob_player_item(player, mob_item_used);
    // `this.heal(foodProperties != null ? healingFactor * foodProperties.nutrition() : defaultHeal)`
    // (`:138`).
    let healed = match nutrition {
        Some(nutrition) => healing_factor * nutrition as f32,
        None => default_heal,
    };
    let Some(target) = store.get_mut(target_id) else {
        return TamingInteraction::Pass;
    };
    let Some(health) = target
        .living()
        .map(crate::entity::living::LivingEntityState::health)
    else {
        return TamingInteraction::Pass;
    };
    if health > 0.0 {
        crate::entity::living::set_health(target, health + healed);
    }
    // `this.playEatingSound()` (`:139`).
    if let (Some(sound), Some(position)) = (eat_sound, store.get(target_id).map(Entity::position)) {
        super::sound_variants::play(store, sound, position, SoundSource::Neutral, 1.0, 1.0);
    }
    TamingInteraction::Fed
}

/// `FoodProperties.nutrition` of the held stack, or `None` for a stack that
/// carries no `FOOD` component. `Err(())` is a patch that names the component
/// in a shape this build cannot read.
pub(super) fn food_nutrition(stack: &ItemStackSummary) -> Result<Option<i32>, ()> {
    let food_type = bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_FOOD_TYPE_ID;
    let stack = bbb_server_types::item_stack::RuntimeItemStack::from_summary(stack.clone());
    let Some(component) = stack
        .effective_component(food_type, &bbb_registry_codec::UnavailableHolderResolver)
        .map_err(|_| ())?
    else {
        return Ok(None);
    };
    component.food_nutrition.map(Some).ok_or(())
}

/// `this.setOrderedToSit(!this.isOrderedToSit())` (`Wolf.java:475`,
/// `Cat.java:421`).
fn toggle_sit(store: &mut EntityStore, target_id: i32) -> TamingInteraction {
    let Some(target) = store.get_mut(target_id) else {
        return TamingInteraction::Pass;
    };
    let EntityData::GeneralLiving(data) = &mut target.data else {
        return TamingInteraction::Pass;
    };
    let Some(tamable) = data.tamable_retained_mut() else {
        return TamingInteraction::Pass;
    };
    let ordered_to_sit = tamable.ordered_to_sit();
    tamable.set_ordered_to_sit(!ordered_to_sit);
    target.mark_synched_dirty(VANILLA_TAMABLE_ANIMAL_FLAGS_DATA_ID);
    TamingInteraction::SitToggled
}

#[cfg(test)]
mod tests;
