//! `Entity.interact`'s leash branches (`Entity.java:2168-2242`) and
//! `LeadItem.bindPlayerMobs` (`LeadItem.java:37-65`).
//!
//! [`after_entity_interact_pass`] is also where the `mobInteract` overrides
//! with an owner hang off `Mob.interact` (`Mob.java:1105-1114`): sheep
//! shearing, the two milk branches, and `Animal`'s feeding branch.
//!
//! The interacting session lives outside the store, so the caller hands its
//! facts in and applies what the answer names: the game event, the server
//! swing and the mutated hand stack. `LeashFenceKnotEntity.interact`'s
//! transfer lives in the same relationship owner.

use bbb_protocol::item_types::{
    VANILLA_ITEM_LEAD_ID, VANILLA_ITEM_NAME_TAG_ID, VANILLA_ITEM_SHEARS_ID,
};
use bbb_protocol::packets::{BlockPos, ItemStackSummary, SoundSource, Vec3d};
use uuid::Uuid;

mod creeper;
mod eligibility;
mod happy_ghast;
mod iron_golem;
mod knot;

use super::{
    aabb_of_size, drop_leash_store_spawn, leash_data, set_leashed_to, set_leashed_to_with,
    LeashAttachment,
};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData};

use crate::entity::block_drops::BlockDropTables;

/// `SoundEvents.LEAD_TIED` (`SoundEvents.java:920`).
const LEAD_TIED_SOUND: &str = "minecraft:item.lead.tied";

/// `SoundEvents.LEAD_UNTIED` (`SoundEvents.java:919`).
const LEAD_UNTIED_SOUND: &str = "minecraft:item.lead.untied";

/// `SoundEvents.SHEARS_SNIP` (`SoundEvents.java:1417`).
const SHEARS_SNIP_SOUND: &str = "minecraft:item.shears.snip";

/// The interacting session as the leash branches read it.
#[derive(Debug)]
pub struct LeashInteractingPlayer<'a> {
    /// `Player.getItemInHand(hand)`; the lead branch shrinks it (`:2237`).
    pub hand_stack: &'a mut ItemStackSummary,
    /// The interaction reached `Mob.usePlayerItem` (`Mob.java:1154-1164`).
    pub mob_item_used: bool,
    pub entity_id: i32,
    pub uuid: Uuid,
    /// `Entity.position()`, the player's feet.
    pub position: Vec3d,
    /// `Player.isSecondaryUseActive()` (`Player.java:291-293`).
    pub secondary_use_active: bool,
    /// `Player.hasInfiniteMaterials()` (`Player.java:1293-1295`).
    pub infinite_materials: bool,
}

pub struct EntityInteractionLoot<'a> {
    pub weather: crate::loot_runtime::LootWeather,
    pub interacting_entity: crate::loot_runtime::LootEntity<'a>,
    pub tables: BlockDropTables<'a>,
    pub dimension: Option<&'a str>,
    pub level: Option<&'a crate::loot_runtime::LootBlockStateAt<'a>>,
    pub block_entity_data: Option<&'a crate::loot_runtime::LootBlockEntityDataAt<'a>>,
    pub item_environment: Option<&'a dyn crate::loot_runtime::LootItemEnvironment>,
    pub enchantment_registry: &'a [String],
    /// The registry-tag generation `Animal.isFood` reads. A level-backed
    /// caller passes the generation it captured, so a food tag edited by a
    /// datapack decides the feeding; without one the branch refuses rather than
    /// guess.
    pub registry_tags: &'a crate::entity::level::EntityRegistryTagView,
    /// `Level.random` (`Level.java:1043-1045`), which the goat's and the
    /// sniffer's `playEatingSound` draw their pitch from
    /// (`Goat.java:193-203`, `Sniffer.java:386-388`).
    pub level_random: Option<&'a crate::random::GameRandom>,
    pub growth: Option<&'a crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'a>>,
    /// The parent's level reads a spawn egg's offspring takes.
    pub spawn_egg: Option<&'a crate::entity::spawn_egg_offspring::SpawnEggLevel>,
}

/// What `Entity.interact`'s leash branches answered.
#[derive(Debug, Clone, PartialEq)]
pub enum LeashInteraction {
    /// `InteractionResult.PASS`: the target's own interact body proceeds.
    Pass,
    /// The bind branch consumed (`:2186-2191`): the caller owes the
    /// `ENTITY_ACTION` game event at the target and the server swing
    /// (`SUCCESS_SERVER`).
    BoundHeld,
    /// A knot transferred leashables; emit `BLOCK_ATTACH` with a client swing.
    KnotTransferred,
    /// `:2206-2220`: the caller owes the `ENTITY_INTERACT` game event at the
    /// target; the swing is the client's own (`SUCCESS`).
    Unleashed,
    /// `:2227-2238`: the caller owes the server swing; the held stack shrank.
    AttachedToPlayer,
    /// `NameTagItem` SUCCESS (`NameTagItem.java:29`) through `Mob.interact`
    /// (`Mob.java:1100-1103`): the caller owes `ENTITY_INTERACT` at the
    /// target; the swing is the client's own (`SUCCESS`).
    Named,
    /// `SpawnEggItem.spawnOffspringFromSpawnEgg` present (`SpawnEggItem.java:151-179`)
    /// through `Mob.interact` (`Mob.java:1100-1103`): `SUCCESS_SERVER`, so the
    /// caller owes `ENTITY_INTERACT` and the server swing.
    SpawnedOffspring,
    /// A leash connection was cut; emit `SHEAR` at the target.
    Sheared,
    /// Equipment was removed; emit `SHEAR` and evaluate its advancement.
    ShearedEquipment {
        entity_id: i32,
        item: bbb_server_types::item_stack::RuntimeItemStack,
    },
    /// `Sheep.mobInteract` returned `CONSUME` because the sheep was a baby or
    /// already sheared (`Sheep.java:141-148`).
    Consumed,
    /// Mob shearing succeeded; emit `SHEAR` and damage the held shears.
    ShearedMob {
        server_swing: bool,
    },
    BrushedArmadillo,
    /// `AbstractCow.mobInteract` SUCCESS (`AbstractCow.java:95`): the caller
    /// owes `ENTITY_INTERACT` (`Mob.java:1110-1111`); no server swing, no `SHEAR`.
    Milked,
    /// A bowl produces one stew, including in creative mode.
    MooshroomStew {
        item: ItemStackSummary,
    },
    /// A living Bucketable mob supplies the components of its filled bucket.
    BucketedMob {
        item: ItemStackSummary,
    },
    /// A brown mooshroom accepted a flower interaction.
    MooshroomFed,
    /// A taming arm that took: the animal now has an owner
    /// (`TamableAnimal.java:168-174`). `server_swing` separates the two
    /// results the arms return -- `Wolf.mobInteract`'s `SUCCESS_SERVER`
    /// (`Wolf.java:494`) from `Cat.mobInteract`'s `SUCCESS` (`Cat.java:435`).
    Tamed {
        server_swing: bool,
        remainder: Option<ItemStackSummary>,
    },
    /// The same two arms with the `nextInt(3)` roll refused (`Wolf.java:507-509`,
    /// `Cat.java:479-480`): the food is still spent and the click still
    /// consumed, and vanilla returns the same result either way.
    TamingFailed {
        server_swing: bool,
        remainder: Option<ItemStackSummary>,
    },
    /// `Animal.mobInteract`'s `SUCCESS_SERVER` (`Animal.java:143-148`): an
    /// adult animal took the 600-tick love timer and ate one food item. The
    /// caller owes `ENTITY_INTERACT` and the server swing.
    FellInLove,
    /// `Animal.mobInteract`'s `SUCCESS` (`:150-155`): a baby animal ate one
    /// food item and grew. The caller owes `ENTITY_INTERACT`; the swing is the
    /// client's own.
    GrewUp,
    /// `Dolphin.mobInteract` (`Dolphin.java:303-318`).
    DolphinFed,
    /// The axolotl's `ItemUtils.createFilledResult` offered a water bucket
    /// (`Axolotl.java:540-546`); the session applies it before the game event.
    FedWithRemainder {
        server_swing: bool,
        item: ItemStackSummary,
    },
    /// `AgeableMob.mobInteract`'s `SUCCESS` (`AgeableMob.java:70-72`): a golden
    /// dandelion flipped `AgeLocked`. The caller owes `ENTITY_INTERACT`; the
    /// swing is the client's own.
    AgeLockToggled,
    /// `Wolf.mobInteract`'s and `Cat.mobInteract`'s collar arm
    /// (`Wolf.java:485-489`, `Cat.java:400-409`): `SUCCESS`, so the caller owes
    /// `ENTITY_INTERACT` and the swing is the client's own.
    CollarDyed,
    /// A feeding override returned `SUCCESS`, including healing a tame animal
    /// or a bee taking a special flower.
    AnimalFed,
    /// Food accepted with `SUCCESS_SERVER`.
    AnimalFedByServer,
    /// A panda stood up from its back with `SUCCESS`.
    PandaStoodUp,
    /// `IronGolem.mobInteract` returns `SUCCESS` after repairing damage.
    Repaired,
    CopperGolemReturnedItem,
    CopperGolemWeatherChanged(crate::entity::copper_golem::WeatherChange),
    /// Ignition succeeded; emit `ENTITY_INTERACT` and apply tool damage if needed.
    IgnitedCreeper {
        damage_tool: bool,
    },
    /// `Parrot.mobInteract`'s poisonous food (`Parrot.java:290-297`): SUCCESS.
    ParrotPoisoned {
        entity_id: i32,
    },
    /// `AbstractHorse.mobInteract`'s ride arm (`AbstractHorse.java:666-667`)
    /// over `doPlayerRide` (`:494-502`): the horse's two body writes are done
    /// and the caller owes the rotation copy and `player.startRiding(this)`
    /// (`:498-500`). The pig's and strider's ride arm owes no rotation
    /// (`Pig.java:159-164`). `SUCCESS`, so the caller owes `ENTITY_INTERACT`
    /// and the swing is the client's own.
    Mounted {
        vehicle_entity_id: i32,
        rotation: Option<(f32, f32)>,
    },
    /// `Horse.mobInteract`'s untamed non-food arm (`Horse.java:164-167`) over
    /// `makeMad` (`AbstractHorse.java:702-707`), also `SUCCESS`.
    MadeMad,
    /// `AbstractHorse.fedFood` (`AbstractHorse.java:413-420`): `SUCCESS_SERVER`,
    /// so the caller owes `ENTITY_INTERACT` and the server swing.
    MountFed,
    /// A saddle or body armour went onto a horse (`AbstractHorse.java:653-663`):
    /// `SUCCESS`.
    MountEquipped {
        entity_id: i32,
        effects: crate::entity::item_slots::SlotWriteEffects,
    },
    /// `AbstractHorse.openCustomInventoryScreen` (`AbstractHorse.java:407-411`):
    /// the caller opens the mount's inventory menu.
    OpenMountInventory {
        vehicle_entity_id: i32,
    },
    /// The sit toggle behind `super.mobInteract` (`Wolf.java:475-479`,
    /// `Cat.java:421-422`). The wolf's is `SUCCESS.withoutItem()` and the
    /// cat's a plain `SUCCESS`; both owe `ENTITY_INTERACT` and no server
    /// swing, and `withoutItem` only suppresses the client's own item
    /// animation.
    SitToggled,
    /// `ZombieVillager.mobInteract`'s golden apple (`ZombieVillager.java:170-179`):
    /// the caller owes `ENTITY_INTERACT`, and the server swing when the cure
    /// `started` (`SUCCESS_SERVER`) rather than refused for want of weakness
    /// (`CONSUME`).
    ZombieVillagerCure {
        started: bool,
    },
    /// A branch whose owner is missing would have run; the caller does
    /// nothing, and the click changes nothing.
    Refused,
}

/// `Leashable.leashableInArea(entity, l -> l.getLeashHolder() == holder)`
/// (`Leashable.java:349-364`): a 32-block box, membership by bounding-box
/// intersection against the canonical retained holder.
fn held_leashables_in_area(store: &EntityStore, holder_id: i32, center: Vec3d) -> Vec<i32> {
    let area = aabb_of_size(center, 32.0);
    store
        .ids_in_box_sections(area)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|entity| {
                area.intersects(entity.bounding_box())
                    && leash_data(entity).and_then(|data| data.leash_holder) == Some(holder_id)
            })
        })
        .collect()
}

fn play_lead_sound(store: &mut EntityStore, position: Vec3d, resource_id: &str) {
    store.play_entity_sound(PendingEntitySound {
        position,
        resource_id: resource_id.to_owned(),
        source: SoundSource::Neutral,
        volume: 1.0,
        pitch: 1.0,
    });
}

/// `Entity.shearOffAllLeashConnections` (`Entity.java:2244-2250`).
pub(crate) fn shear_off_all_leash_connections(
    store: &mut EntityStore,
    target_id: i32,
    target_center: Vec3d,
    target_position: Vec3d,
) -> bool {
    let dropped = drop_all_leash_connections(store, target_id, target_center);
    if dropped {
        // `:2247`: `player.getSoundSource()` is `PLAYERS` (`Player.java:402-404`).
        store.play_entity_sound(PendingEntitySound {
            position: target_position,
            resource_id: SHEARS_SNIP_SOUND.to_owned(),
            source: SoundSource::Players,
            volume: 1.0,
            pitch: 1.0,
        });
    }
    dropped
}

/// `Entity.dropAllLeashConnections` (`Entity.java:2253-2270`).
pub(crate) fn drop_all_leash_connections(
    store: &mut EntityStore,
    target_id: i32,
    target_center: Vec3d,
) -> bool {
    // `Leashable.leashableLeashedTo` (`Leashable.java:349-351`) over the same
    // 32-block `leashableInArea` scan `held_leashables_in_area` already owns.
    let inbound = held_leashables_in_area(store, target_id, target_center);
    let target_leashed = store.get(target_id).is_some_and(super::is_leashed);
    if inbound.is_empty() && !target_leashed {
        return false;
    }
    let mut dropped = false;
    // `:2256-2263`: every cut is `dropLeash()` (`Leashable.java:111-113`),
    // which always `spawnAtLocation(LEAD)`. Creative only skips `hurtAndBreak`.
    if target_leashed {
        if let Some(mut target) = store.detach(target_id) {
            if drop_leash_store_spawn(&mut target, store, true, true) {
                dropped = true;
            }
            store.attach(target);
        }
    }
    for id in inbound {
        let Some(mut entity) = store.detach(id) else {
            continue;
        };
        if drop_leash_store_spawn(&mut entity, store, true, true) {
            dropped = true;
        }
        store.attach(entity);
    }
    dropped
}

fn held_is(stack: &ItemStackSummary, item_id: i32) -> bool {
    stack.item_id == Some(item_id)
}

/// `Player.getBoundingBox().getCenter()` through the standing catalog box.
fn player_center(position: Vec3d) -> Vec3d {
    let [_, height] = bbb_protocol::entity_types::vanilla_entity_type_dimensions(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    )
    .expect("the player is a built-in type");
    Vec3d {
        x: position.x,
        y: position.y + f64::from(height) / 2.0,
        z: position.z,
    }
}

/// The leash branches of `Entity.interact` (`Entity.java:2168-2242`) for one
/// stored target the world already range-checked. Standalone callers get the
/// generated base Vanilla tags for `Animal.isFood`; a level-backed caller
/// hands its own generation to [`interact_leash_with_loot`].
pub fn interact_leash(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
) -> LeashInteraction {
    interact_leash_inner(
        store,
        target_id,
        player,
        None,
        &LeashEnv {
            registry_tags: &crate::entity::level::EntityRegistryTagView::Builtin26_1,
            thundering: false,
            level_random: None,
            growth: None,
            spawn_egg: None,
        },
    )
}

pub fn interact_leash_with_loot(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
    loot: EntityInteractionLoot<'_>,
) -> LeashInteraction {
    let EntityInteractionLoot {
        weather,
        dimension,
        interacting_entity,
        tables,
        level,
        block_entity_data,
        item_environment,
        enchantment_registry,
        registry_tags,
        level_random,
        growth,
        spawn_egg,
    } = loot;
    let mut loot = crate::entity::shearing_loot::ShearingLoot {
        tables,
        dimension,
        weather,
        interacting_entity,
        level,
        block_entity_data,
        item_environment,
        enchantment_registry,
    };
    interact_leash_inner(
        store,
        target_id,
        player,
        Some(&mut loot),
        &LeashEnv {
            registry_tags,
            thundering: weather.thundering,
            level_random,
            growth,
            spawn_egg,
        },
    )
}

#[derive(Clone, Copy)]
struct LeashEnv<'a> {
    registry_tags: &'a crate::entity::level::EntityRegistryTagView,
    thundering: bool,
    level_random: Option<&'a crate::random::GameRandom>,
    growth: Option<&'a crate::entity::living::dimensions_growth::SizeGrowthCollisionView<'a>>,
    spawn_egg: Option<&'a crate::entity::spawn_egg_offspring::SpawnEggLevel>,
}

fn interact_leash_inner(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
    shearing_loot: Option<&mut crate::entity::shearing_loot::ShearingLoot<'_>>,
    env: &LeashEnv<'_>,
) -> LeashInteraction {
    let LeashEnv {
        registry_tags,
        thundering: _,
        level_random,
        growth: _,
        spawn_egg,
    } = *env;
    // ZombieHorse.java:57-60, CamelHusk.java:40-43.
    if let Some(target) = store.get_mut(target_id).filter(|target| {
        matches!(
            target.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
        )
    }) {
        crate::entity::conversion::set_persistence_required(target);
    }
    let Some(target) = store.get(target_id) else {
        return LeashInteraction::Pass;
    };
    if matches!(target.data(), EntityData::LeashKnot(_)) {
        return knot::interact(store, target_id, player);
    }
    let target_is_mob = matches!(target.data(), EntityData::GeneralLiving(_));
    let target_position = target.position();
    let target_center = target.bounding_box().center();
    let target_alive = target.is_alive_with_health();
    if target_is_mob && !target_alive {
        return LeashInteraction::Pass;
    }
    let bind = eligibility::can_be_leashed(target, store)
        && target_alive
        && !matches!(target.data(), EntityData::GeneralLiving(data) if data.is_baby());
    // `Mob.interact` (`Mob.java:1097-1098`) never reaches this when dead.
    let target_living_alive =
        target_is_mob && crate::entity::name_tag::living_entity_is_alive(target);
    // `Mob.checkAndHandleImportantInteractions` (`Mob.java:1121-1145`) runs
    // before the leash branches. A named name tag consumes; an unnamed tag
    // PASSes so the spawn-egg check and then `super.interact` can run.
    if target_is_mob {
        if target_living_alive && held_is(player.hand_stack, VANILLA_ITEM_NAME_TAG_ID) {
            match crate::entity::name_tag::interact_living_entity(
                store,
                target_id,
                player.hand_stack,
            ) {
                crate::entity::name_tag::NameTagInteraction::Success => {
                    return LeashInteraction::Named
                }
                crate::entity::name_tag::NameTagInteraction::Pass => {}
            }
        }
        if target_living_alive
            && crate::entity::spawn_egg_offspring::held_is_spawn_egg(player.hand_stack)
        {
            let random = level_random.map_or_else(|| store.random().clone(), Clone::clone);
            match crate::entity::spawn_egg_offspring::spawn_offspring_from_spawn_egg(
                store,
                target_id,
                crate::entity::spawn_egg_offspring::SpawnEggUser {
                    hand_stack: player.hand_stack,
                    infinite_materials: player.infinite_materials,
                    uuid: player.uuid,
                },
                spawn_egg,
                registry_tags,
                &random,
            ) {
                crate::entity::spawn_egg_offspring::SpawnEggOffspring::Spawned => {
                    return LeashInteraction::SpawnedOffspring;
                }
                crate::entity::spawn_egg_offspring::SpawnEggOffspring::Pass => {}
            }
        }
    }

    // `:2169-2192`, the bind branch.
    if player.secondary_use_active && bind {
        let candidates = held_leashables_in_area(store, player.entity_id, target_center);
        let mut any_leashed = false;
        for id in candidates {
            let Some(mut candidate) = store.detach(id) else {
                continue;
            };
            if eligibility::can_attach(&candidate, store, target_id, target_center)
                && set_leashed_to(&mut candidate, store, target_id, true)
            {
                any_leashed = true;
            }
            store.attach(candidate);
        }
        if any_leashed {
            play_lead_sound(store, target_position, LEAD_TIED_SOUND);
            return LeashInteraction::BoundHeld;
        }
    }

    // `:2195-2197` over `shearOffAllLeashConnections` (`:2244-2250`).
    if held_is(player.hand_stack, VANILLA_ITEM_SHEARS_ID) {
        if shear_off_all_leash_connections(store, target_id, target_center, target_position) {
            return LeashInteraction::Sheared;
        }
        // `:2198-2203` over `attemptToShearEquipment` (`:2273-2295`).
        if target_is_mob
            && !player.secondary_use_active
            && crate::entity::shear_equipment::can_shear_equipment(store, target_id, player.uuid)
        {
            let sheared = crate::entity::shear_equipment::attempt_to_shear_equipment(
                store,
                target_id,
                player.infinite_materials,
            );
            if let Some(item) = sheared {
                return LeashInteraction::ShearedEquipment {
                    entity_id: target_id,
                    item,
                };
            }
            // `:2202`: the call sits in the `else if`; false falls through
            // to leash/PASS, then `mobInteract` (`Mob.java:1105-1114`).
        }
    }

    if !target_alive {
        return LeashInteraction::Pass;
    }

    let holder = store
        .get(target_id)
        .and_then(|target| leash_data(target))
        .map(|data| (data.leash_holder, data.holder_is_player));
    // `:2206-2220`.
    if holder.is_some_and(|(id, _)| id == Some(player.entity_id)) {
        let Some(mut target) = store.detach(target_id) else {
            return LeashInteraction::Pass;
        };
        // `:2209-2211`: `hasInfiniteMaterials` removes, anyone else drops.
        let dropped = drop_leash_store_spawn(&mut target, store, true, !player.infinite_materials);
        if dropped {
            // `:2215`.
            play_lead_sound(store, target_position, LEAD_UNTIED_SOUND);
        }
        store.attach(target);
        return if dropped {
            LeashInteraction::Unleashed
        } else {
            LeashInteraction::Refused
        };
    }
    // `:2222-2239`.
    if held_is(player.hand_stack, VANILLA_ITEM_LEAD_ID)
        && !holder.is_some_and(|(id, is_player)| id.is_some() && is_player)
        && store.get(target_id).is_some_and(|target| {
            eligibility::can_attach(
                target,
                store,
                player.entity_id,
                player_center(player.position),
            )
        })
    {
        let Some(mut target) = store.detach(target_id) else {
            return LeashInteraction::Pass;
        };
        // `:2228-2230`.
        if leash_data(&target).is_some_and(|data| data.leash_holder.is_some())
            && !drop_leash_store_spawn(&mut target, store, true, true)
        {
            store.attach(target);
            return LeashInteraction::Refused;
        }
        // `:2232-2237`.
        let attached = set_leashed_to_with(
            &mut target,
            store,
            player.entity_id,
            LeashAttachment::Holder(player.uuid),
            true,
            true,
        );
        if attached {
            play_lead_sound(store, target_position, LEAD_TIED_SOUND);
        }
        store.attach(target);
        if attached {
            // `itemStack.shrink(1)` (`:2237`), unconditional of the game
            // mode.
            player.hand_stack.count -= 1;
            if player.hand_stack.count <= 0 {
                *player.hand_stack = ItemStackSummary::empty();
            }
            return LeashInteraction::AttachedToPlayer;
        }
        return LeashInteraction::Refused;
    }
    after_entity_interact_pass(store, target_id, player, shearing_loot, env)
}

/// `Mob.interact` after `super.interact` PASSed (`Mob.java:1105-1114`).
fn after_entity_interact_pass(
    store: &mut EntityStore,
    target_id: i32,
    player: &mut LeashInteractingPlayer<'_>,
    mut shearing_loot: Option<&mut crate::entity::shearing_loot::ShearingLoot<'_>>,
    env: &LeashEnv<'_>,
) -> LeashInteraction {
    let LeashEnv {
        registry_tags,
        thundering,
        level_random,
        growth,
        ..
    } = *env;
    if let Some(outcome) = happy_ghast::mob_interact(store, target_id, player, registry_tags) {
        return outcome;
    }
    if let Some(item) = store
        .get(target_id)
        .and_then(|target| crate::entity::bucketable::pickup(target, player.hand_stack))
    {
        return LeashInteraction::BucketedMob { item };
    }
    if let Some(damage_tool) = creeper::mob_interact(store, target_id, player, registry_tags) {
        return LeashInteraction::IgnitedCreeper { damage_tool };
    }
    if iron_golem::mob_interact(store, target_id, player) {
        return LeashInteraction::Repaired;
    }
    match crate::entity::dolphin::mob_interact(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        registry_tags,
        growth,
    ) {
        crate::entity::dolphin::Feeding::Pass => {}
        crate::entity::dolphin::Feeding::Fed => return LeashInteraction::DolphinFed,
        crate::entity::dolphin::Feeding::Refused => return LeashInteraction::Refused,
    }
    match crate::entity::zombie::villager::mob_interact(
        store,
        target_id,
        player.hand_stack,
        player.uuid,
        player.infinite_materials,
    ) {
        crate::entity::zombie::villager::CureInteraction::Pass => {}
        crate::entity::zombie::villager::CureInteraction::Started => {
            return LeashInteraction::ZombieVillagerCure { started: true };
        }
        crate::entity::zombie::villager::CureInteraction::Refused => {
            return LeashInteraction::ZombieVillagerCure { started: false };
        }
    }
    if crate::entity::armadillo::brush(
        store,
        target_id,
        player.hand_stack,
        shearing_loot.as_deref_mut(),
    ) {
        return LeashInteraction::BrushedArmadillo;
    }
    if crate::entity::copper_golem::return_held_item(
        store,
        target_id,
        player.hand_stack,
        player.position,
    ) {
        return LeashInteraction::CopperGolemReturnedItem;
    }
    if crate::entity::bogged::shear(
        store,
        target_id,
        player.hand_stack,
        SoundSource::Players,
        shearing_loot.as_deref_mut(),
    ) || crate::entity::snow_golem::shear(
        store,
        target_id,
        player.hand_stack,
        SoundSource::Players,
        shearing_loot.as_deref_mut(),
    ) || crate::entity::copper_golem::shear(
        store,
        target_id,
        player.hand_stack,
        SoundSource::Players,
        registry_tags,
    ) {
        return LeashInteraction::ShearedMob {
            server_swing: false,
        };
    }
    if let Some(change) =
        crate::entity::copper_golem::weather_interaction(store, target_id, player, registry_tags)
    {
        return LeashInteraction::CopperGolemWeatherChanged(change);
    }
    match crate::entity::sheep_shear::mob_interact(
        store,
        target_id,
        player.hand_stack,
        SoundSource::Players,
        shearing_loot.as_deref_mut(),
    ) {
        crate::entity::sheep_shear::SheepInteraction::Pass => {}
        crate::entity::sheep_shear::SheepInteraction::Consumed => {
            return LeashInteraction::Consumed;
        }
        crate::entity::sheep_shear::SheepInteraction::Sheared => {
            return LeashInteraction::ShearedMob { server_swing: true };
        }
    }
    if crate::entity::mooshroom::shear(
        store,
        target_id,
        player.hand_stack,
        SoundSource::Players,
        shearing_loot,
    ) {
        return LeashInteraction::ShearedMob {
            server_swing: false,
        };
    }
    if let Some(outcome) = crate::entity::mooshroom::mob_interact(
        store,
        target_id,
        player.hand_stack,
        player.infinite_materials,
    ) {
        return outcome;
    }
    if crate::entity::cow_milk::mob_interact(
        store,
        target_id,
        player.entity_id,
        player.position,
        player.hand_stack,
        player.infinite_materials,
    ) {
        return LeashInteraction::Milked;
    }
    if crate::entity::goat_milk::mob_interact(
        store,
        target_id,
        player.entity_id,
        player.position,
        player.hand_stack,
        player.infinite_materials,
    ) {
        return LeashInteraction::Milked;
    }
    let nautilus = crate::entity::nautilus_interact::mob_interact(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        player.secondary_use_active,
        registry_tags,
        growth,
        &mut player.mob_item_used,
    );
    if !matches!(nautilus, LeashInteraction::Pass) {
        return nautilus;
    }
    // `Wolf.mobInteract` and `Cat.mobInteract` run their own arms before
    // calling `super.mobInteract` (`Wolf.java:448-490`, `Cat.java:399-417`), so
    // the taming module is asked ahead of the feeding one.
    match crate::entity::taming::mob_interact(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        registry_tags,
        &mut player.mob_item_used,
    ) {
        crate::entity::taming::TamingInteraction::Pass => {}
        crate::entity::taming::TamingInteraction::Tamed => {
            return LeashInteraction::Tamed {
                server_swing: taming_arm_swings_on_the_server(store, target_id),
                remainder: None,
            };
        }
        crate::entity::taming::TamingInteraction::TamingFailed => {
            return LeashInteraction::TamingFailed {
                server_swing: taming_arm_swings_on_the_server(store, target_id),
                remainder: None,
            };
        }
        crate::entity::taming::TamingInteraction::CollarDyed => {
            return LeashInteraction::CollarDyed
        }
        crate::entity::taming::TamingInteraction::Fed => return LeashInteraction::AnimalFed,
        crate::entity::taming::TamingInteraction::PoisonousFood => {
            return LeashInteraction::ParrotPoisoned {
                entity_id: target_id,
            }
        }
        crate::entity::taming::TamingInteraction::SitToggled => {
            return LeashInteraction::SitToggled
        }
        crate::entity::taming::TamingInteraction::ArmourEquipped(effects) => {
            return LeashInteraction::MountEquipped {
                entity_id: target_id,
                effects,
            }
        }
        crate::entity::taming::TamingInteraction::ArmourRepaired => {
            return LeashInteraction::Repaired
        }
        crate::entity::taming::TamingInteraction::Refused => return LeashInteraction::Refused,
    }
    // `Horse.mobInteract` runs its own arms before `super.mobInteract`
    // (`Horse.java:155-170`), so the ride module is asked ahead of the feeding
    // one, the same way the taming module is.
    match crate::entity::horse_ride::mob_interact(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        player.secondary_use_active,
        registry_tags,
        growth,
    ) {
        crate::entity::horse_ride::HorseInteraction::Pass => {}
        crate::entity::horse_ride::HorseInteraction::Fed => return LeashInteraction::MountFed,
        crate::entity::horse_ride::HorseInteraction::Equipped(effects) => {
            return LeashInteraction::MountEquipped {
                entity_id: target_id,
                effects,
            }
        }
        crate::entity::horse_ride::HorseInteraction::OpenInventory => {
            return LeashInteraction::OpenMountInventory {
                vehicle_entity_id: target_id,
            }
        }
        crate::entity::horse_ride::HorseInteraction::Mounted { y_rot, x_rot } => {
            return LeashInteraction::Mounted {
                vehicle_entity_id: target_id,
                rotation: Some((y_rot, x_rot)),
            };
        }
        crate::entity::horse_ride::HorseInteraction::MadeMad => return LeashInteraction::MadeMad,
        crate::entity::horse_ride::HorseInteraction::Refused => return LeashInteraction::Refused,
    }
    match crate::entity::item_steering::ride_arm(
        store,
        target_id,
        player.hand_stack,
        player.secondary_use_active,
        registry_tags,
    ) {
        crate::entity::item_steering::SteerableInteraction::Mounted => {
            return LeashInteraction::Mounted {
                vehicle_entity_id: target_id,
                rotation: None,
            };
        }
        crate::entity::item_steering::SteerableInteraction::Refused => {
            return LeashInteraction::Refused
        }
        _ => {}
    }
    if let Some(outcome) = crate::entity::panda::mob_interact(
        store,
        target_id,
        player,
        registry_tags,
        thundering,
        growth,
    ) {
        return outcome;
    }
    // Each of these is one subtype's `mobInteract` (or `Animal`'s own), so at
    // most one of them matches the target's type.
    let fed = match crate::entity::animal_feeding::mob_interact(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        registry_tags,
        level_random,
        &mut player.mob_item_used,
        growth,
    ) {
        crate::entity::animal_feeding::AnimalFeeding::Pass => None,
        crate::entity::animal_feeding::AnimalFeeding::FellInLove => {
            Some(LeashInteraction::FellInLove)
        }
        crate::entity::animal_feeding::AnimalFeeding::GrewUp => Some(LeashInteraction::GrewUp),
        crate::entity::animal_feeding::AnimalFeeding::Fed => Some(LeashInteraction::AnimalFed),
        crate::entity::animal_feeding::AnimalFeeding::FedWithRemainder { server_swing, item } => {
            Some(LeashInteraction::FedWithRemainder { server_swing, item })
        }
        crate::entity::animal_feeding::AnimalFeeding::AgeLockToggled => {
            Some(LeashInteraction::AgeLockToggled)
        }
        crate::entity::animal_feeding::AnimalFeeding::Refused => return LeashInteraction::Refused,
    };
    // The tail both tamable overrides run once `super.mobInteract` answered
    // (`Wolf.java:473-482`, `Cat.java:419-441`).
    match crate::entity::taming::after_super_mob_interact(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        registry_tags,
        fed.is_some(),
    ) {
        crate::entity::taming::TamingInteraction::SitToggled => {
            return LeashInteraction::SitToggled
        }
        crate::entity::taming::TamingInteraction::Refused => return LeashInteraction::Refused,
        _ => {}
    }
    if let Some(fed) = fed {
        return fed;
    }
    match crate::entity::item_steering::saddle_arm(
        store,
        target_id,
        &mut crate::entity::animal_feeding::FeedingPlayer {
            hand_stack: &mut *player.hand_stack,
            uuid: player.uuid,
            infinite_materials: player.infinite_materials,
        },
        registry_tags,
    ) {
        crate::entity::item_steering::SteerableInteraction::Saddled(effects) => {
            LeashInteraction::MountEquipped {
                entity_id: target_id,
                effects,
            }
        }
        crate::entity::item_steering::SteerableInteraction::Refused => LeashInteraction::Refused,
        _ => LeashInteraction::Pass,
    }
}

/// `Wolf.mobInteract`'s taming arm answers `SUCCESS_SERVER` (`Wolf.java:494`)
/// and `Cat.mobInteract`'s answers `SUCCESS` (`Cat.java:435`); only the first
/// owes the server swing.
fn taming_arm_swings_on_the_server(store: &EntityStore, target_id: i32) -> bool {
    store
        .get(target_id)
        .is_some_and(|target| target.data().entity_resource_id() == "minecraft:wolf")
}

/// What `LeadItem.bindPlayerMobs` answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadBindOutcome {
    /// `InteractionResult.PASS` (`LeadItem.java:40`, `:64`).
    Pass,
    /// `SUCCESS_SERVER` (`:54-57`): the caller owes the `BLOCK_ATTACH` game
    /// event at the fence and the server swing.
    Bound,
}

/// `LeadItem.bindPlayerMobs` (`LeadItem.java:37-65`); the caller already
/// proved the clicked state `#minecraft:fences`.
pub fn bind_player_mobs(
    store: &mut EntityStore,
    player_entity_id: i32,
    pos: BlockPos,
) -> LeadBindOutcome {
    // `Vec3.atCenterOf(pos)` (`:38`).
    let center = Vec3d {
        x: f64::from(pos.x) + 0.5,
        y: f64::from(pos.y) + 0.5,
        z: f64::from(pos.z) + 0.5,
    };
    let candidates = held_leashables_in_area(store, player_entity_id, center);
    if candidates.is_empty() {
        return LeadBindOutcome::Pass;
    }
    // `:43-44` over `getKnot` (`LeashFenceKnotEntity.java:130-142`) and
    // `createKnot` (`:144-149`).
    let (knot_id, fresh_knot) = match super::find_knot(store, pos) {
        Some(id) => (id, false),
        None => match crate::entity::hanging::spawn_leash_knot(store, pos) {
            Some(id) => (id, true),
            None => return LeadBindOutcome::Pass,
        },
    };
    let Some(knot_center) = store.get(knot_id).map(|knot| knot.bounding_box().center()) else {
        return LeadBindOutcome::Pass;
    };
    let mut any_leashed = false;
    for id in candidates {
        let Some(mut candidate) = store.detach(id) else {
            continue;
        };
        if eligibility::can_attach(&candidate, store, knot_id, knot_center)
            && set_leashed_to(&mut candidate, store, knot_id, true)
        {
            any_leashed = true;
        }
        store.attach(candidate);
    }
    if any_leashed {
        // `activeKnot.playPlacementSound()` (`:55`,
        // `LeashFenceKnotEntity.java:150-152`).
        if let Some(knot_position) = store.get(knot_id).map(Entity::position) {
            play_lead_sound(store, knot_position, LEAD_TIED_SOUND);
        }
        return LeadBindOutcome::Bound;
    }
    // `:60-62`: `discard` runs the removal callback at once (`Entity.java:3852-3863`).
    if fresh_knot {
        store.remove(knot_id, crate::entity::RemovalReason::Discarded);
    }
    LeadBindOutcome::Pass
}
