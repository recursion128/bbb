//! `EntityPredicate.matches` over the entity a key carries.
//!
//! The first false term answers (`EntityPredicate.matches`), so a later term
//! is never read. A term whose value the caller did not supply is refused,
//! never answered false: `None` from a read below is an
//! unknown value, and the two answers vanilla reaches without a value — a null
//! vehicle and a failed `instanceof` — are [`LootEntityAnswer::NotApplicable`].

use std::borrow::Cow;

use bbb_facts::persistent_registry_facts_26_1::vanilla_persistent_registry_tag_members_26_1;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_server_datapack::loot::{
    DistancePredicate, EntityFlags, EntityPredicate, EntitySubPredicate, ItemPredicate,
    LocationPredicate, MovementPredicate,
};

use super::item_predicate::{
    check_component_predicates, check_item_predicate, component_predicates_match,
    item_predicate_matches, ComponentReads,
};
use super::location_predicate::{check_location_predicate, location_predicate_matches_in_world};
use super::{
    malformed, unsupported_field, LootBlockStateAt, LootEntity, LootEntityAnswer, LootEntityFlags,
    LootOrigin, LootRuntimeReason,
};

/// The `ORIGIN` the `distance` term measures from and the level the
/// `location` term reads.
#[derive(Clone, Copy, Default)]
pub(super) struct EntityPredicateWorld<'a> {
    pub(super) origin: Option<LootOrigin>,
    pub(super) dimension: Option<&'a str>,
    pub(super) level: Option<&'a LootBlockStateAt<'a>>,
    pub(super) block_entity_data: Option<&'a super::LootBlockEntityDataAt<'a>>,
    pub(super) biome: Option<&'a super::LootBiomeAt<'a>>,
    pub(super) light: Option<&'a super::LootLightAt<'a>>,
    pub(super) structures: Option<&'a super::LootStructuresAt<'a>>,
    pub(super) sky: Option<&'a super::LootSkyAt<'a>>,
    pub(super) item_environment: Option<&'a dyn super::LootItemEnvironment>,
    pub(super) item_resolver: Option<&'a dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
    pub(super) vehicle_slots: Option<&'a super::params::LootVehicleSlots<'a>>,
    pub(super) bodies: Option<crate::entity::store::EntityBodyReadView<'a>>,
}

fn resolve_facts_movement_block(
    view: LootEntity<'_>,
    world: EntityPredicateWorld<'_>,
) -> Option<[i32; 3]> {
    let LootEntity::Facts(facts) = view else {
        return None;
    };
    let position = facts.position?;
    let supporting = facts
        .supporting_block
        .map(|[x, y, z]| bbb_protocol::packets::BlockPos { x, y, z });
    let mut known = true;
    let pos = crate::entity::body_on_pos(
        bbb_protocol::packets::Vec3d {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        supporting,
        0.500_001_f32,
        |pos| {
            let Some(state_id) = world.level.and_then(|blocks| blocks(pos)) else {
                known = false;
                return crate::entity::SupportingBlockFacts {
                    state_id: 0,
                    is_fence: false,
                    is_wall: false,
                };
            };
            let resolver = world.item_resolver.or_else(|| {
                world
                    .item_environment
                    .map(super::LootItemEnvironment::item_resolver)
            });
            let is_wall =
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state_id)
                    .and_then(bbb_protocol::block_types::vanilla_block_type)
                    .and_then(|block| {
                        resolver
                            .and_then(|resolver| {
                                resolver.tag_contains(
                                    "minecraft:block",
                                    "minecraft:walls",
                                    block.resource_id,
                                )
                            })
                            .or_else(|| {
                                match vanilla_persistent_registry_tag_members_26_1(
                                    "minecraft:block",
                                    "minecraft:walls",
                                ) {
                                    PersistentRegistryTagMembers::Present(members) => Some(
                                        members.iter().any(|member| *member == block.resource_id),
                                    ),
                                    PersistentRegistryTagMembers::Absent => Some(false),
                                    PersistentRegistryTagMembers::Unavailable => None,
                                }
                            })
                    });
            let Some(is_wall) = is_wall else {
                known = false;
                return crate::entity::SupportingBlockFacts {
                    state_id,
                    is_fence: false,
                    is_wall: false,
                };
            };
            crate::entity::SupportingBlockFacts {
                state_id,
                is_fence: false,
                is_wall,
            }
        },
    );
    known.then_some([pos.x, pos.y, pos.z])
}

fn refuse(detail: impl Into<String>) -> LootRuntimeReason {
    unsupported_field("entity_properties", detail.into())
}

/// Everything a walk without an entity can refuse about the predicate.
pub(super) fn check_entity_predicate(
    predicate: &EntityPredicate,
) -> std::result::Result<(), LootRuntimeReason> {
    check_component_predicates(&predicate.components, &predicate.predicates)?;
    if let Some(EntitySubPredicate::Unsupported(id)) = &predicate.type_specific {
        return Err(refuse(format!(
            "predicate.type_specific {id} is a sub-predicate with its own reads"
        )));
    }
    for location in [
        &predicate.location,
        &predicate.movement_affected_by,
        &predicate.stepping_on,
    ]
    .into_iter()
    .flatten()
    {
        check_location_predicate(location).map_err(location_predicate_refusal)?;
    }
    for (_, item) in predicate.equipment.iter().flatten() {
        check_item_predicate(item).map_err(item_predicate_refusal)?;
    }
    for (_, item) in predicate.slots.iter().flatten() {
        check_item_predicate(item).map_err(|reason| match reason {
            LootRuntimeReason::UnsupportedField { detail, .. } => {
                refuse(format!("predicate.slots.{detail}"))
            }
            reason => reason,
        })?;
    }
    for related in [
        &predicate.vehicle,
        &predicate.passenger,
        &predicate.targeted_entity,
    ]
    .into_iter()
    .flatten()
    {
        check_entity_predicate(related)?;
    }
    if let Some(EntitySubPredicate::Lightning {
        entity_struck: Some(struck),
        ..
    }) = &predicate.type_specific
    {
        check_entity_predicate(struck)?;
    }
    if let Some(EntitySubPredicate::Player { terms, .. }) = &predicate.type_specific {
        if let Some(looking_at) = &terms.looking_at {
            check_entity_predicate(looking_at)?;
        }
    }
    Ok(())
}

/// `EntityPredicate.matches` for an entity that *is* present. An empty
/// predicate reads nothing, so it is the only one an opaque entity answers.
pub(super) fn entity_predicate_matches(
    predicate: &EntityPredicate,
    entity: LootEntity<'_>,
    world: EntityPredicateWorld<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    check_entity_predicate(predicate)?;
    if predicate.is_empty() {
        return Ok(true);
    }
    let Some(view) = entity.readable() else {
        return Err(refuse(
            "a predicate term needs the entity's values and this caller has none",
        ));
    };
    predicate_matches(predicate, view, world)
}

fn live_entity_view<'a>(view: LootEntity<'a>, world: &EntityPredicateWorld<'a>) -> LootEntity<'a> {
    match view {
        LootEntity::Facts(facts) if facts.entity_uuid.is_some() && facts.player_uuid.is_none() => {
            world
                .item_environment
                .and_then(|environment| environment.entity_for_relations(view, world.bodies))
                .map_or(view, LootEntity::Stored)
        }
        _ => view,
    }
}

fn predicate_matches(
    predicate: &EntityPredicate,
    view: LootEntity<'_>,
    world: EntityPredicateWorld<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    if let Some(types) = &predicate.entity_type {
        let entity_type = view
            .entity_type()
            .ok_or_else(|| refuse("predicate.type needs a view that names the entity's type"))?;
        if !types.contains(entity_type, |tag| {
            entity_type_tag_contains(tag, entity_type, world.item_resolver)
        })? {
            return Ok(false);
        }
    }
    if let Some(distance) = &predicate.distance {
        if !distance_match(distance, view, world)? {
            return Ok(false);
        }
    }
    if let Some(movement) = &predicate.movement {
        if !movement_match(movement, view)? {
            return Ok(false);
        }
    }
    if let Some(location) = &predicate.location {
        let position = view.position().ok_or_else(|| {
            refuse("predicate.location needs a view that names the entity's position")
        })?;
        if !location_match(location, [position.x, position.y, position.z], world)? {
            return Ok(false);
        }
    }
    if let Some(location) = &predicate.movement_affected_by {
        // Tested at the centre of `getBlockPosBelowThatAffectsMyMovement`
        // (`EntityPredicate.java:129-134`, `Entity.java:988-1007`).
        let [x, y, z] = view
            .block_pos_below_that_affects_movement()
            .or_else(|| resolve_facts_movement_block(view, world))
            .ok_or_else(|| {
                refuse(
                    "predicate.movement_affected_by needs a view that names the block below the entity",
                )
            })?;
        if !location_match(location, [x, y, z].map(|axis| f64::from(axis) + 0.5), world)? {
            return Ok(false);
        }
    }
    if let Some(location) = &predicate.stepping_on {
        let on_ground = view.flags().on_ground.ok_or_else(|| {
            refuse("predicate.stepping_on needs a view that supplies is_on_ground")
        })?;
        if on_ground {
            let pos = view.on_pos().ok_or_else(|| {
                refuse("predicate.stepping_on needs a view that supplies the supporting position")
            })?;
            if !location_match(location, pos.map(|axis| f64::from(axis) + 0.5), world)? {
                return Ok(false);
            }
        } else {
            return Ok(false);
        }
    }
    if let Some(flags) = &predicate.flags {
        if !flags_match(flags, view)? {
            return Ok(false);
        }
    }
    if let Some(expected) = &predicate.nbt {
        let environment = world
            .item_environment
            .ok_or_else(|| refuse("predicate.nbt needs the entity's actual save owner"))?;
        let nbt_view = match view {
            LootEntity::Facts(facts) if facts.nbt.is_none() => live_entity_view(view, &world),
            _ => view,
        };
        let actual = environment.nbt_entity(nbt_view)?;
        if !bbb_server_types::block_state_parse::block_entity_data_matches(
            Some(expected),
            Some(&actual),
        ) {
            return Ok(false);
        }
    }
    if let Some(equipment) = &predicate.equipment {
        if !equipment_match(equipment, view, world.item_resolver)? {
            return Ok(false);
        }
    }
    if let Some(slots) = &predicate.slots {
        let slot_view = live_entity_view(view, &world);
        // `SlotsPredicate.matches` / `matchSlots` (`SlotsPredicate.java:15-34`).
        for (range, predicate) in slots {
            if let LootEntity::Stored(stored) = slot_view {
                if crate::entity::item_slots::container_has_loot(stored.entity) {
                    let size = crate::entity::item_slots::container_size(stored.entity)
                        .expect("a pending vehicle has cargo slots");
                    if range
                        .iter()
                        .any(|slot| usize::try_from(*slot).is_ok_and(|slot| slot < size))
                    {
                        let read = world.vehicle_slots.ok_or_else(|| {
                            refuse("predicate.slots needs vehicle loot at its actual level")
                        })?;
                        read(stored.entity, range)?;
                    }
                }
            }
            let mut any = false;
            for slot in range {
                if let Some((tool, resolver)) =
                    slot_view.slot_item(*slot, world.item_environment)?
                {
                    if item_predicate_matches(predicate, &tool, resolver)? {
                        any = true;
                        break;
                    }
                }
            }
            if !any {
                return Ok(false);
            }
        }
    }
    if let Some(effects) = &predicate.effects {
        let living = view
            .living()
            .ok_or_else(|| refuse("predicate.effects needs a living entity view"))?;
        if !living
            || !crate::entity::living::mob_effects::mob_effects_predicate_matches(effects, |id| {
                view.active_effect(id)
                    .ok_or_else(|| refuse("predicate.effects needs the entity active effects"))
            })?
        {
            return Ok(false);
        }
    }
    if let Some(sub) = &predicate.type_specific {
        if !sub_predicate_match(sub, view, world)? {
            return Ok(false);
        }
    }
    if let Some(vehicle) = &predicate.vehicle {
        let term = match view.vehicle() {
            None => {
                return Err(refuse(
                    "predicate.vehicle needs a view that can look a vehicle up",
                ))
            }
            Some(LootEntityAnswer::NotApplicable) => false,
            // The vehicle is measured from the same origin.
            Some(LootEntityAnswer::Value(entity)) => predicate_matches(vehicle, entity, world)?,
        };
        if !term {
            return Ok(false);
        }
    }
    let relation_view = match view {
        LootEntity::Facts(_)
            if predicate.passenger.is_some() || predicate.targeted_entity.is_some() =>
        {
            world
                .item_environment
                .and_then(|environment| environment.entity_for_relations(view, world.bodies))
                .map(LootEntity::Stored)
                .ok_or_else(|| refuse("predicate relationships need the entity's live body"))?
        }
        _ => view,
    };
    if let Some(passenger) = &predicate.passenger {
        let passengers = relation_view.passengers().ok_or_else(|| {
            refuse("predicate.passenger needs the entity's live passenger relationships")
        })?;
        let mut any = false;
        // EntityPredicate.java:156-158: only direct passengers, from the same origin.
        for entity in passengers {
            let entity = entity
                .ok_or_else(|| refuse("predicate.passenger needs the passenger's live entity"))?;
            if predicate_matches(passenger, entity, world)? {
                any = true;
                break;
            }
        }
        if !any {
            return Ok(false);
        }
    }
    if let Some(targeted) = &predicate.targeted_entity {
        let term = match relation_view.targeted_entity() {
            None => {
                return Err(refuse(
                    "predicate.targeted_entity needs the mob's live target",
                ))
            }
            Some(LootEntityAnswer::NotApplicable) => false,
            Some(LootEntityAnswer::Value(entity)) => predicate_matches(targeted, entity, world)?,
        };
        if !term {
            return Ok(false);
        }
    }
    if let Some(expected) = &predicate.team {
        let environment = world
            .item_environment
            .ok_or_else(|| refuse("predicate.team needs the actual scoreboard"))?;
        if !environment.entity_team_matches(view, expected, world.bodies)? {
            return Ok(false);
        }
    }
    if let Some(period) = predicate.periodic_tick {
        let tick_count = view.tick_count().ok_or_else(|| {
            refuse("predicate.periodic_tick needs a view that reads the tick count")
        })?;
        if tick_count % period != 0 {
            return Ok(false);
        }
    }
    if !predicate.components.is_empty() || !predicate.predicates.is_empty() {
        let component_view = live_entity_view(view, &world);
        let resolver = world
            .item_resolver
            .or_else(|| {
                world
                    .item_environment
                    .map(super::LootItemEnvironment::item_resolver)
            })
            .or_else(|| match component_view {
                LootEntity::Stored(stored) => {
                    stored.bodies.map(|bodies| bodies.store().item_resolver())
                }
                _ => None,
            })
            .ok_or_else(|| refuse("entity components need their registry publication"))?;
        let get = |type_id| {
            component_view
                .component(type_id)?
                .map(|value| {
                    bbb_registry_codec::read_item_component(type_id, &value, resolver)
                        .map(Cow::Owned)
                        .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
                })
                .transpose()
        };
        let has = |type_id| Ok(component_view.component(type_id)?.is_some());
        if !component_predicates_match(
            &predicate.components,
            &predicate.predicates,
            &ComponentReads {
                get: &get,
                has: &has,
            },
            None,
            resolver,
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn sub_predicate_match(
    sub: &EntitySubPredicate,
    view: LootEntity<'_>,
    world: EntityPredicateWorld<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    match sub {
        // The `instanceof` first, then the optional field.
        EntitySubPredicate::Sheep { sheared } => Ok(match (view.sheared(), sheared) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(actual), Some(expected)) => actual == *expected,
        }),
        EntitySubPredicate::Slime { size } => match view.slime_size() {
            None => Err(refuse(
                "predicate.type_specific minecraft:slime needs a view that reads a slime's size",
            )),
            Some(LootEntityAnswer::NotApplicable) => Ok(false),
            Some(LootEntityAnswer::Value(actual)) => Ok(size.matches(actual)),
        },
        EntitySubPredicate::Raider {
            has_raid,
            is_captain,
        } => raider_match(*has_raid, *is_captain, view),
        // `FishingHookPredicate.matches` (`FishingHookPredicate.java:29-35`).
        EntitySubPredicate::FishingHook { in_open_water } => {
            Ok(match (in_open_water, view.fishing_hook_open_water()) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(expected), Some(actual)) => actual == *expected,
            })
        }
        // `LightningBoltPredicate.matches` (`LightningBoltPredicate.java:31-36`).
        EntitySubPredicate::Lightning {
            blocks_set_on_fire,
            entity_struck,
        } => {
            let Some((fires, struck)) = view.lightning() else {
                return Ok(false);
            };
            if !blocks_set_on_fire.matches(fires) {
                return Ok(false);
            }
            match entity_struck {
                None => Ok(true),
                Some(predicate) => {
                    for entity in struck {
                        if predicate_matches(predicate, entity, world)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
            }
        }
        // `PlayerPredicate.matches` (`PlayerPredicate.java:74-137`).
        EntitySubPredicate::Player {
            level,
            food_level,
            food_saturation,
            gamemode,
            terms,
        } => {
            let Some(player) = view.player() else {
                return Ok(false);
            };
            if !(level.matches(player.experience_level)
                && food_level.matches(player.food_level)
                && food_saturation.matches(f64::from(player.saturation_level))
                && gamemode
                    .as_ref()
                    .is_none_or(|names| names.iter().any(|name| name == player.game_mode)))
            {
                return Ok(false);
            }
            if !terms.reads_records() && terms.looking_at.is_none() {
                return Ok(true);
            }
            let uuid = match view {
                LootEntity::Stored(stored) => Some(stored.entity.uuid()),
                LootEntity::Facts(facts) => facts.player_uuid,
                _ => None,
            }
            .ok_or_else(|| {
                refuse("predicate.type_specific minecraft:player needs the player's uuid")
            })?;
            let environment = world.item_environment.ok_or_else(|| {
                refuse("predicate.type_specific minecraft:player needs the online player")
            })?;
            if terms.reads_records() && !environment.player_terms_match(uuid, terms)? {
                return Ok(false);
            }
            if let Some(looking_at) = &terms.looking_at {
                let Some(target) = environment.player_looking_at(uuid)? else {
                    return Ok(false);
                };
                let world = EntityPredicateWorld {
                    bodies: target.bodies,
                    ..world
                };
                return predicate_matches(looking_at, LootEntity::Stored(target), world);
            }
            Ok(true)
        }
        EntitySubPredicate::Unsupported(id) => Err(refuse(format!(
            "predicate.type_specific {id} is a sub-predicate with its own reads"
        ))),
    }
}

/// `EntityFlagsPredicate.matches`: `is_flying`, `is_fall_flying` and
/// `is_baby` are gated on `instanceof LivingEntity`, so that gate is read only
/// when one of them is spelled.
fn flags_match(
    terms: &EntityFlags,
    view: LootEntity<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    if *terms == EntityFlags::default() {
        return Ok(true);
    }
    let gated = terms.flying.is_some() || terms.fall_flying.is_some() || terms.baby.is_some();
    let living = if gated {
        view.living().ok_or_else(|| {
            unsupported_field(
                "entity_properties",
                "predicate.flags needs a view that says whether the entity is a living one"
                    .to_string(),
            )
        })?
    } else {
        false
    };
    flags_terms_match(terms, living, view.flags())
}

/// Every spelled term is read, so a flag no view answers refuses even when an
/// earlier term already failed.
fn flags_terms_match(
    terms: &EntityFlags,
    living: bool,
    flags: LootEntityFlags,
) -> std::result::Result<bool, LootRuntimeReason> {
    let flag = |term: Option<bool>, value: Option<bool>, name: &str| match (term, value) {
        (None, _) => Ok(true),
        (Some(_), None) => Err(unsupported_field(
            "entity_properties",
            format!("predicate.flags.{name} is a flag this view does not read"),
        )),
        (Some(expected), Some(actual)) => Ok(actual == expected),
    };
    if !flag(terms.on_ground, flags.on_ground, "is_on_ground")? {
        return Ok(false);
    }
    if !flag(terms.on_fire, flags.on_fire, "is_on_fire")? {
        return Ok(false);
    }
    if !flag(terms.crouching, flags.crouching, "is_sneaking")? {
        return Ok(false);
    }
    if !flag(terms.sprinting, flags.sprinting, "is_sprinting")? {
        return Ok(false);
    }
    if !flag(terms.swimming, flags.swimming, "is_swimming")? {
        return Ok(false);
    }
    // `is_flying` is `isFallFlying() || abilities.flying`; a non-living entity
    // is not flying.
    if let Some(expected) = terms.flying {
        let actual = if living {
            let fall_flying = flags.fall_flying.ok_or_else(|| {
                unsupported_field(
                    "entity_properties",
                    "predicate.flags.is_flying is a flag this view does not read".to_string(),
                )
            })?;
            let ability_flying = flags.ability_flying.ok_or_else(|| {
                unsupported_field(
                    "entity_properties",
                    "predicate.flags.is_flying is a flag this view does not read".to_string(),
                )
            })?;
            fall_flying || ability_flying
        } else {
            false
        };
        if actual != expected {
            return Ok(false);
        }
    }
    if !flag(terms.in_water, flags.in_water, "is_in_water")? {
        return Ok(false);
    }
    // A non-living entity passes both terms whatever they spell.
    if living {
        if !flag(terms.fall_flying, flags.fall_flying, "is_fall_flying")? {
            return Ok(false);
        }
        if !flag(terms.baby, flags.baby, "is_baby")? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `EntityEquipmentPredicate.matches`: the `instanceof LivingEntity` gate,
/// then one `ItemPredicate.test` per spelled slot.
fn equipment_match(
    equipment: &[(EquipmentSlot, ItemPredicate)],
    view: LootEntity<'_>,
    resolver: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let living = view.living().ok_or_else(|| {
        refuse("predicate.equipment needs a view that says whether the entity is a living one")
    })?;
    if !living {
        return Ok(false);
    }
    for (slot, predicate) in equipment {
        let tool = view.equipment_item(*slot).ok_or_else(|| {
            refuse(format!(
                "predicate.equipment needs a view that reads the {slot:?} slot"
            ))
        })?;
        if !item_predicate_matches(
            predicate,
            &tool,
            resolver.unwrap_or(&bbb_registry_codec::UnavailableHolderResolver),
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Item-predicate refusals re-attributed to the equipment term.
fn item_predicate_refusal(reason: LootRuntimeReason) -> LootRuntimeReason {
    match reason {
        LootRuntimeReason::MalformedField { .. } => {
            malformed("entity_properties", "predicate.equipment")
        }
        LootRuntimeReason::UnsupportedField { detail, .. } => {
            refuse(format!("predicate.equipment.{detail}"))
        }
        reason => reason,
    }
}

/// Location-predicate refusals re-attributed to the entity term.
fn location_predicate_refusal(reason: LootRuntimeReason) -> LootRuntimeReason {
    match reason {
        LootRuntimeReason::MalformedField { .. } => {
            malformed("entity_properties", "predicate.location")
        }
        LootRuntimeReason::UnsupportedField { detail, .. } => {
            refuse(format!("predicate.location {detail}"))
        }
        reason => reason,
    }
}

fn location_match(
    predicate: &LocationPredicate,
    [x, y, z]: [f64; 3],
    world: EntityPredicateWorld<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    location_predicate_matches_in_world(
        predicate,
        [x, y, z],
        world.dimension,
        world.level,
        world.block_entity_data,
        world.biome,
        world.sky,
        world.light,
        world.structures,
        world.item_resolver,
        world.item_environment,
    )
    .map_err(location_predicate_refusal)
}

/// One entity-type tag from the loaded datapacks, or from the generated 26.1
/// facts when the caller has no registry view.
fn entity_type_tag_contains(
    tag_id: &str,
    entity_type: &str,
    registries: Option<&dyn bbb_registry_codec::PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let members = match registries {
        Some(registries) => registries.tag_members("minecraft:entity_type", tag_id),
        None => vanilla_persistent_registry_tag_members_26_1("minecraft:entity_type", tag_id),
    };
    match members {
        PersistentRegistryTagMembers::Present(members) => {
            Ok(members.iter().any(|member| *member == entity_type))
        }
        // A tag nothing declares is an empty holder set, which is what
        // `HolderSet.contains` answers for it.
        PersistentRegistryTagMembers::Absent => Ok(false),
        PersistentRegistryTagMembers::Unavailable => Err(unsupported_field(
            "entity_properties",
            format!("entity-type tag #{tag_id} has no generated membership"),
        )),
    }
}

/// `DistancePredicate.matches` between the origin and the entity, over the
/// `float` differences vanilla compares. A spelled `distance` with no origin
/// is false without reading the entity.
fn distance_match(
    distance: &DistancePredicate,
    view: LootEntity<'_>,
    world: EntityPredicateWorld<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let Some(origin) = world.origin else {
        return Ok(false);
    };
    let position = view.position().ok_or_else(|| {
        refuse("predicate.distance needs a view that names the entity's position")
    })?;
    Ok(distance.matches_delta([
        (origin.x - position.x) as f32,
        (origin.y - position.y) as f32,
        (origin.z - position.z) as f32,
    ]))
}

/// `MovementPredicate.matches` over the per-tick movement scaled to blocks
/// per second.
fn movement_match(
    movement: &MovementPredicate,
    view: LootEntity<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let known = view.movement().ok_or_else(|| {
        refuse("predicate.movement needs a view that reads the entity's known movement")
    })?;
    let [x, y, z] = known.known_movement.map(|axis| axis * 20.0);
    Ok(movement.x.matches(x)
        && movement.y.matches(y)
        && movement.z.matches(z)
        && movement.speed.matches_sqr(x * x + y * y + z * z)
        && movement.horizontal_speed.matches_sqr(x * x + z * z)
        && movement.vertical_speed.matches(y.abs())
        && movement.fall_distance.matches(known.fall_distance))
}

/// `RaiderPredicate.matches`: the `instanceof Raider` gate, then both flags
/// equal. An answered flag that disagrees decides false; only otherwise does a
/// flag the view left open refuse.
fn raider_match(
    has_raid: bool,
    is_captain: bool,
    view: LootEntity<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    match view.raider_flags() {
        None => Err(unsupported_field(
            "entity_properties",
            "predicate.type_specific minecraft:raider needs a view that reads a raider's raid and captain flags"
                .to_string(),
        )),
        Some(LootEntityAnswer::NotApplicable) => Ok(false),
        Some(LootEntityAnswer::Value(actual)) => {
            if actual.has_raid == Some(!has_raid) || actual.is_captain == Some(!is_captain) {
                return Ok(false);
            }
            match (actual.has_raid, actual.is_captain) {
                (Some(_), Some(_)) => Ok(true),
                (None, _) => Err(unsupported_field(
                    "entity_properties",
                    "predicate.type_specific minecraft:raider needs has_raid, which reads the level's raids"
                        .to_string(),
                )),
                (_, None) => Err(unsupported_field(
                    "entity_properties",
                    "predicate.type_specific minecraft:raider needs is_captain, which this view does not read"
                        .to_string(),
                )),
            }
        }
    }
}

#[cfg(test)]
mod tests;
