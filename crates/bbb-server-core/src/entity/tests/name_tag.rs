use super::*;

mod johnny;

use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::item_types::{
    VANILLA_ITEM_COW_SPAWN_EGG_ID, VANILLA_ITEM_CREEPER_SPAWN_EGG_ID, VANILLA_ITEM_NAME_TAG_ID,
    VANILLA_ITEM_PIG_SPAWN_EGG_ID, VANILLA_ITEM_ZOMBIE_SPAWN_EGG_ID,
};
use bbb_protocol::RichTextComponent;

use crate::entity::leash::{self, LeashInteractingPlayer};
use crate::entity::store::EntityIdCounter;

const PLAYER_ENTITY_ID: i32 = 10_000;

fn pig_position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn spawn_pig(store: &mut EntityStore) -> i32 {
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_PIG_ID, pig_position())
        .expect("a stored pig")
}

fn interacting_player(hand_stack: &mut ItemStackSummary) -> LeashInteractingPlayer<'_> {
    LeashInteractingPlayer {
        mob_item_used: false,
        hand_stack,
        entity_id: PLAYER_ENTITY_ID,
        uuid: uuid::Uuid::from_u128(1),
        position: Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
        secondary_use_active: false,
        infinite_materials: false,
    }
}

fn named_name_tag(count: i32, name: &str) -> ItemStackSummary {
    let component = RichTextComponent::plain(name);
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_NAME_TAG_ID),
        count,
        component_patch: DataComponentPatchSummary {
            custom_name: Some(name.to_string()),
            custom_name_styled: Some(component.styled().to_vec()),
            custom_name_nbt: Some(component.component().clone()),
            ..DataComponentPatchSummary::default()
        }
        .into(),
    }
}

fn blank_name_tag() -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(VANILLA_ITEM_NAME_TAG_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn pig_custom_name(store: &EntityStore, id: i32) -> Option<RichTextComponent> {
    store
        .get(id)
        .and_then(crate::entity::Entity::custom_name)
        .cloned()
}

fn pig_persistence_required(store: &EntityStore, id: i32) -> bool {
    match store.get(id).map(crate::entity::Entity::data) {
        Some(EntityData::GeneralLiving(data)) => data
            .mob_retained()
            .is_some_and(|mob| mob.persistence_required()),
        _ => false,
    }
}

/// `NameTagItem.interactLivingEntity` (`NameTagItem.java:17-29`) on a live
/// serializable mob, then `Mob.interact` (`Mob.java:1100-1103`).
#[test]
fn a_named_name_tag_renames_a_live_mob_and_owes_entity_interact() {
    let mut store = EntityStore::new(0x7a6, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    let mut hand = named_name_tag(1, "Babe");

    let outcome = leash::interact_leash(&mut store, pig, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::Named);
    assert_eq!(
        pig_custom_name(&store, pig)
            .as_ref()
            .map(|name| name.text()),
        Some("Babe")
    );
    assert!(pig_persistence_required(&store, pig));
    assert_eq!(hand.count, 0);
}

/// `Player.interactOn` (`Player.java:836-838`) restores a creative shrink.
#[test]
fn infinite_materials_restores_the_shrunk_name_tag() {
    let mut store = EntityStore::new(0x7a7, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    let mut hand = named_name_tag(1, "Babe");
    let original = hand.clone();
    let outcome = {
        let mut player = interacting_player(&mut hand);
        player.infinite_materials = true;
        leash::interact_leash(&mut store, pig, &mut player)
    };
    // `Player.interactOn` (`Player.java:836-838`) restores the creative shrink.
    if hand.count < original.count {
        hand = original;
    }

    assert_eq!(outcome, leash::LeashInteraction::Named);
    assert_eq!(
        pig_custom_name(&store, pig)
            .as_ref()
            .map(|name| name.text()),
        Some("Babe")
    );
    assert!(pig_persistence_required(&store, pig));
    assert_eq!(hand.count, 1);
    assert_eq!(hand.item_id, Some(VANILLA_ITEM_NAME_TAG_ID));
}

/// `:31`: a tag with no `CUSTOM_NAME` PASSes; `super.interact` leash may run.
#[test]
fn a_name_tag_without_a_custom_name_passes_through_to_leash() {
    let mut store = EntityStore::new(0x7a8, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    let mut hand = blank_name_tag();

    let outcome = leash::interact_leash(&mut store, pig, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(pig_custom_name(&store, pig), None);
    assert!(!pig_persistence_required(&store, pig));
    assert_eq!(hand.count, 1);
}

/// `Mob.interact` (`Mob.java:1097-1098`) never reaches the name-tag item.
#[test]
fn a_named_name_tag_does_not_rename_a_dead_mob() {
    let mut store = EntityStore::new(0x7a9, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    crate::entity::living::set_health(store.get_mut(pig).expect("the pig is filed"), 0.0);
    let mut hand = named_name_tag(1, "Babe");

    let outcome = leash::interact_leash(&mut store, pig, &mut interacting_player(&mut hand));

    assert_ne!(outcome, leash::LeashInteraction::Named);
    assert_eq!(pig_custom_name(&store, pig), None);
    assert!(!pig_persistence_required(&store, pig));
    assert_eq!(hand.count, 1);
}

fn spawn_egg(item_id: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: DataComponentPatchSummary::default().into(),
    }
}

fn living_ids_of_type(store: &EntityStore, entity_type_id: i32) -> Vec<i32> {
    store
        .ids()
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|entity| {
                entity.entity_type_id() == entity_type_id && entity.is_alive()
            })
        })
        .collect()
}

fn ageable_is_baby(store: &EntityStore, id: i32) -> bool {
    match store.get(id).map(crate::entity::Entity::data) {
        Some(EntityData::GeneralLiving(data)) => data
            .ageable_retained()
            .is_some_and(|ageable| ageable.is_baby()),
        _ => false,
    }
}

fn zombie_is_baby(store: &EntityStore, id: i32) -> bool {
    match store.get(id).map(crate::entity::Entity::data) {
        Some(EntityData::GeneralLiving(data)) => data
            .zombie_retained()
            .is_some_and(|zombie| zombie.is_baby()),
        _ => false,
    }
}

/// `SpawnEggItem.spawnOffspringFromSpawnEgg` (`SpawnEggItem.java:151-179`) on a
/// matching AgeableMob: a baby of the same type at the parent, stack consumed.
#[test]
fn a_matching_pig_spawn_egg_spawns_a_baby_and_shrinks() {
    let mut store = EntityStore::new(0x7aa, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    let mut hand = spawn_egg(VANILLA_ITEM_PIG_SPAWN_EGG_ID);

    let outcome = leash::interact_leash(&mut store, pig, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::SpawnedOffspring);
    let pigs = living_ids_of_type(&store, VANILLA_ENTITY_TYPE_PIG_ID);
    assert_eq!(pigs.len(), 2);
    let baby = *pigs.iter().find(|id| **id != pig).expect("the offspring");
    assert!(ageable_is_baby(&store, baby));
    assert!(!ageable_is_baby(&store, pig));
    assert_eq!(
        store.get(baby).map(crate::entity::Entity::position),
        Some(pig_position())
    );
    assert_eq!(hand.count, 0);
}

/// Type mismatch is empty → `PASS`, so leash may still run.
#[test]
fn a_cow_spawn_egg_on_a_pig_passes_without_a_baby() {
    let mut store = EntityStore::new(0x7ab, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    let mut hand = spawn_egg(VANILLA_ITEM_COW_SPAWN_EGG_ID);

    let outcome = leash::interact_leash(&mut store, pig, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        living_ids_of_type(&store, VANILLA_ENTITY_TYPE_PIG_ID).len(),
        1
    );
    assert_eq!(hand.count, 1);
    assert_eq!(hand.item_id, Some(VANILLA_ITEM_COW_SPAWN_EGG_ID));
}

/// `Mob.interact` (`Mob.java:1097-1098`) never reaches the spawn-egg item.
#[test]
fn a_spawn_egg_does_not_run_on_a_dead_mob() {
    let mut store = EntityStore::new(0x7ac, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    crate::entity::living::set_health(store.get_mut(pig).expect("the pig is filed"), 0.0);
    let mut hand = spawn_egg(VANILLA_ITEM_PIG_SPAWN_EGG_ID);

    let outcome = leash::interact_leash(&mut store, pig, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        living_ids_of_type(&store, VANILLA_ENTITY_TYPE_PIG_ID).len(),
        1
    );
    assert_eq!(hand.count, 1);
}

/// `ItemStack.consume` skips the shrink when `hasInfiniteMaterials`.
#[test]
fn infinite_materials_does_not_consume_the_spawn_egg() {
    let mut store = EntityStore::new(0x7ad, EntityIdCounter::new());
    let pig = spawn_pig(&mut store);
    let mut hand = spawn_egg(VANILLA_ITEM_PIG_SPAWN_EGG_ID);
    let outcome = {
        let mut player = interacting_player(&mut hand);
        player.infinite_materials = true;
        leash::interact_leash(&mut store, pig, &mut player)
    };

    assert_eq!(outcome, leash::LeashInteraction::SpawnedOffspring);
    assert_eq!(
        living_ids_of_type(&store, VANILLA_ENTITY_TYPE_PIG_ID).len(),
        2
    );
    assert_eq!(hand.count, 1);
    assert_eq!(hand.item_id, Some(VANILLA_ITEM_PIG_SPAWN_EGG_ID));
}

/// `Zombie.setBaby` (`Zombie.java:187-195`) is live, so a matching egg works.
#[test]
fn a_matching_zombie_spawn_egg_spawns_a_baby() {
    let mut store = EntityStore::new(0x7ae, EntityIdCounter::new());
    let zombie = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, pig_position())
        .expect("a stored zombie");
    let mut hand = spawn_egg(VANILLA_ITEM_ZOMBIE_SPAWN_EGG_ID);

    let outcome = leash::interact_leash(&mut store, zombie, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::SpawnedOffspring);
    let zombies = living_ids_of_type(&store, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(zombies.len(), 2);
    let baby = *zombies
        .iter()
        .find(|id| **id != zombie)
        .expect("the offspring");
    assert!(zombie_is_baby(&store, baby));
    assert!(!zombie_is_baby(&store, zombie));
    assert_eq!(hand.count, 0);
}

/// Most monsters keep `isBaby()` false after `setBaby(true)` → empty / Pass.
#[test]
fn a_creeper_spawn_egg_on_a_creeper_passes_without_a_baby() {
    let mut store = EntityStore::new(0x7af, EntityIdCounter::new());
    let creeper = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_CREEPER_ID, pig_position())
        .expect("a stored creeper");
    let mut hand = spawn_egg(VANILLA_ITEM_CREEPER_SPAWN_EGG_ID);

    let outcome = leash::interact_leash(&mut store, creeper, &mut interacting_player(&mut hand));

    assert_eq!(outcome, leash::LeashInteraction::Pass);
    assert_eq!(
        living_ids_of_type(&store, VANILLA_ENTITY_TYPE_CREEPER_ID).len(),
        1
    );
    assert_eq!(hand.count, 1);
}
