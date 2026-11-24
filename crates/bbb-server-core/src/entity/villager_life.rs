//! The villager's household: its food and inventory, breeding, the farmer's
//! field and composter (`Villager.java`, `VillagerMakeLove.java`,
//! `HarvestFarmland.java`, `WorkAtComposter.java`).

use bbb_protocol::block_types::{
    VANILLA_BLOCK_BEETROOTS_ID, VANILLA_BLOCK_CARROTS_ID, VANILLA_BLOCK_PITCHER_CROP_ID,
    VANILLA_BLOCK_POTATOES_ID, VANILLA_BLOCK_TORCHFLOWER_CROP_ID, VANILLA_BLOCK_WHEAT_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_VILLAGER_ID;
use bbb_protocol::item_types::{
    VANILLA_ITEM_BEETROOT_ID, VANILLA_ITEM_BEETROOT_SEEDS_ID, VANILLA_ITEM_BONE_MEAL_ID,
    VANILLA_ITEM_BREAD_ID, VANILLA_ITEM_CARROT_ID, VANILLA_ITEM_PITCHER_POD_ID,
    VANILLA_ITEM_POTATO_ID, VANILLA_ITEM_TORCHFLOWER_SEEDS_ID, VANILLA_ITEM_WHEAT_ID,
    VANILLA_ITEM_WHEAT_SEEDS_ID,
};
use bbb_protocol::packets::{BlockPos, EntityEvent, Vec3d};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::{MemoryGlobalPos, MemoryModuleType, MemoryValue};
use crate::entity::{Entity, EntityData, EntityStore, GeneralLivingData};
use crate::world::simulation_level::LevelCtx;

/// `Villager.BREEDING_FOOD_THRESHOLD` (`Villager.java:99`).
const BREEDING_FOOD_THRESHOLD: i32 = 12;
/// `AgeableMob.setAge(6000)` of both parents and `-24000` of the child
/// (`VillagerMakeLove.java:107-109`).
const PARENT_AGE: i32 = 6000;
const CHILD_AGE: i32 = -24_000;
/// `AcquirePoi.SCAN_RANGE`, the bed search of `takeVacantBed`
/// (`VillagerMakeLove.java:92-94`).
const BED_SEARCH_RANGE: i32 = 48;
const ANGRY_EVENT: i8 = 13;
const HEART_EVENT: i8 = 12;
const FARMER_PROFESSION_ID: i32 = 5;

/// `Villager.FOOD_POINTS` (`Villager.java:100`).
const fn food_points(item_id: i32) -> i32 {
    match item_id {
        VANILLA_ITEM_BREAD_ID => 4,
        VANILLA_ITEM_POTATO_ID | VANILLA_ITEM_CARROT_ID | VANILLA_ITEM_BEETROOT_ID => 1,
        _ => 0,
    }
}

fn data(entity: &Entity) -> Option<&GeneralLivingData> {
    match entity.data() {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

fn data_mut(entity: &mut Entity) -> Option<&mut GeneralLivingData> {
    match &mut entity.data {
        EntityData::GeneralLiving(data) => Some(data),
        _ => None,
    }
}

fn is_villager(entity: &Entity) -> bool {
    entity.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID
}

/// `Villager.onReputationEventFrom` (`Villager.java:861-864`), with the
/// gossip caps from `GossipType.java:7-8`.
pub(crate) fn hurt_reputation(entity: &mut Entity, attacker: uuid::Uuid) -> bool {
    if !is_villager(entity) {
        return false;
    }
    crate::entity::zombie::villager::add_gossip(entity, attacker, "minor_negative", 25, 200);
    true
}

/// `Villager.tellWitnessesThatIWasMurdered` (`Villager.java:614-620`).
pub(crate) fn murder_reputation(
    victim: &Entity,
    murderer: crate::entity::damage::StoredDamageCausingEntity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
) {
    if !is_villager(victim) {
        return;
    }
    let murderer = match murderer {
        crate::entity::damage::StoredDamageCausingEntity::Player { uuid, .. } => Some(uuid),
        _ if murderer.entity_id() == victim.id() => Some(victim.uuid()),
        _ => store
            .get(murderer.entity_id())
            .or_else(|| level.detached_entity(murderer.entity_id()))
            .map(Entity::uuid),
    };
    let Some(murderer) = murderer else {
        return;
    };
    let witnesses = {
        use crate::entity::ai::mob::target::{conditions::TargetingConditions, TargetView};

        // `NearestLivingEntitySensor.java:17-24`, `Sensor.java:52-69`.
        let range = victim.living().map_or(0.0, |living| {
            living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
        });
        let area = victim.bounding_box().inflate(range, range, range);
        let sensing = Default::default();
        let view = TargetView::new(victim, store, level, &sensing);
        let conditions = TargetingConditions::for_non_combat().range(range);
        view.local_stored()
            .filter(|target| {
                target.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID
                    && target.bounding_box().intersects(area)
                    && conditions.test(&view, target, |_| true)
            })
            .map(|target| target.entity_id())
            .collect::<Vec<_>>()
    };
    for id in witnesses {
        if let Some(witness) = store.get_mut(id) {
            crate::entity::zombie::villager::add_gossip(
                witness,
                murderer,
                "major_negative",
                25,
                100,
            );
        }
    }
}

/// `Villager.foodLevel`, saved as the `FoodLevel` byte.
fn food_level(entity: &Entity) -> i32 {
    data(entity)
        .and_then(GeneralLivingData::villager_food_level)
        .map_or(0, i32::from)
}

fn set_food_level(entity: &mut Entity, value: i32) {
    // `output.putByte("FoodLevel", (byte)this.foodLevel)` (`Villager.java:489`).
    if let Some(data) = data_mut(entity) {
        data.set_villager_food_level(value as i8);
    }
}

fn slots(entity: &Entity) -> &[Option<RuntimeItemStack>] {
    data(entity)
        .and_then(GeneralLivingData::internal_inventory)
        .map(crate::entity::living_internal_inventory::LivingInternalInventory::slots)
        .unwrap_or(&[])
}

fn item(entity: &Entity, slot: usize) -> Option<&RuntimeItemStack> {
    data(entity)?.internal_inventory()?.item(slot)
}

fn set_slot(entity: &mut Entity, slot: usize, stack: Option<RuntimeItemStack>) {
    if let Some(inventory) = data_mut(entity).and_then(GeneralLivingData::internal_inventory_mut) {
        inventory.set_item(slot, stack.filter(|stack| stack.count() > 0));
    }
}

/// `SimpleContainer.countItem`.
pub(crate) fn count_item_id(entity: &Entity, item_id: i32) -> i32 {
    slots(entity)
        .iter()
        .flatten()
        .filter(|stack| !stack.is_empty() && stack.item_id() == Some(item_id))
        .map(RuntimeItemStack::count)
        .sum()
}

#[cfg(test)]
pub(crate) fn count_item(entity: &Entity, item: &str) -> i32 {
    bbb_protocol::item_types::vanilla_item_type_for_resource_id(item)
        .map_or(0, |catalog| count_item_id(entity, catalog.id))
}

/// `Villager.countFoodPointsInInventory` (`Villager.java:792-795`).
pub(crate) fn food_points_in_inventory(entity: &Entity) -> i32 {
    slots(entity)
        .iter()
        .flatten()
        .filter(|stack| !stack.is_empty())
        .map(|stack| stack.count() * stack.item_id().map_or(0, food_points))
        .sum()
}

/// `Villager.canBreed` (`Villager.java:644-646`).
pub(crate) fn can_breed(entity: &Entity) -> bool {
    is_villager(entity)
        && food_level(entity) + food_points_in_inventory(entity) >= BREEDING_FOOD_THRESHOLD
        && entity.sleeping_pos().is_none()
        && data(entity)
            .and_then(GeneralLivingData::ageable_retained)
            .is_some_and(|age| age.age() == 0)
}

/// `Villager.eatAndDigestFood` (`Villager.java:682-685`) over `eatUntilFull`
/// (`:652-673`).
fn eat_and_digest_food(entity: &mut Entity) {
    let mut food = food_level(entity);
    for slot in 0..slots(entity).len() {
        if food >= BREEDING_FOOD_THRESHOLD {
            break;
        }
        let Some(stack) = item(entity, slot) else {
            continue;
        };
        let points = stack.item_id().map_or(0, food_points);
        if points == 0 {
            continue;
        }
        let mut stack = stack.clone();
        while stack.count() > 0 && food < BREEDING_FOOD_THRESHOLD {
            food += points;
            stack.shrink(1);
        }
        set_slot(entity, slot, Some(stack));
    }
    set_food_level(entity, food - BREEDING_FOOD_THRESHOLD);
}

/// `SimpleContainer.addItem` (`SimpleContainer.java:71-84`): fill matching
/// stacks, then empty slots; answers what did not fit.
pub(crate) fn add_item(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
    stack: RuntimeItemStack,
) -> RuntimeItemStack {
    let Some(inventory) = data_mut(entity).and_then(GeneralLivingData::internal_inventory_mut)
    else {
        return stack;
    };
    inventory.add_item(level.item_stack_resolver(), stack)
}

/// `SimpleContainer.removeItemType`.
fn remove_item_type(entity: &mut Entity, item_id: i32, mut count: i32) {
    for slot in 0..slots(entity).len() {
        let Some(mut stack) = self::item(entity, slot)
            .filter(|stack| stack.item_id() == Some(item_id))
            .cloned()
        else {
            continue;
        };
        let taken = stack.count().min(count);
        stack.shrink(taken);
        count -= taken;
        set_slot(entity, slot, Some(stack));
        if count == 0 {
            break;
        }
    }
}

/// `Villager.wantsToPickUp` (`Villager.java:778-782`): the
/// `villager_picks_up` tag or the profession's requested items, and room for
/// some of it.
pub(crate) fn wants_to_pick_up(
    entity: &Entity,
    level: &LevelCtx<'_, '_>,
    stack: &RuntimeItemStack,
) -> bool {
    let Some(item_id) = stack.item_id() else {
        return false;
    };
    // `VillagerProfession.FARMER`'s requested items (`VillagerProfession.java:179-183`).
    let requested = is_farmer(entity) && FARMER_REQUESTED_IDS.contains(&item_id);
    if !requested {
        let Some(item) = stack.item_resource_id() else {
            return false;
        };
        if !level.registry_tag_membership("minecraft:item", "minecraft:villager_picks_up", item) {
            return false;
        }
    }
    data(entity)
        .and_then(GeneralLivingData::internal_inventory)
        .is_some_and(|inventory| {
            inventory
                .can_add_item(level.item_stack_resolver(), stack)
                .unwrap_or(false)
        })
}

/// `Villager.hasExcessFood` / `wantsMoreFood` (`Villager.java:784-790`).
fn has_excess_food(entity: &Entity) -> bool {
    food_points_in_inventory(entity) >= 24
}

fn wants_more_food(entity: &Entity) -> bool {
    food_points_in_inventory(entity) < 12
}

fn is_farmer(entity: &Entity) -> bool {
    data(entity)
        .and_then(GeneralLivingData::villager_brain_state)
        .is_some_and(|(profession_id, _, _)| profession_id == FARMER_PROFESSION_ID)
}

/// `VillagerProfession.FARMER`'s requested items, the only profession that
/// names any (`VillagerProfession.java:179-183`).
const FARMER_REQUESTED_IDS: [i32; 4] = [
    VANILLA_ITEM_WHEAT_ID,
    VANILLA_ITEM_WHEAT_SEEDS_ID,
    VANILLA_ITEM_BEETROOT_SEEDS_ID,
    VANILLA_ITEM_BONE_MEAL_ID,
];

/// `TradeWithVillager.tick`'s hand-over once the two stand close
/// (`TradeWithVillager.java:44-62`): spare food, a farmer's spare wheat, and
/// the items the other's profession asks for.
pub(crate) fn trade_with_villager(body: &mut Entity, store: &mut EntityStore, target_id: i32) {
    let Some(target) = store.get(target_id).filter(|target| is_villager(target)) else {
        return;
    };
    let target_position = target.position();
    let target_wants_food = wants_more_food(target);
    // `figureOutWhatIAmWillingToTrade` (`:67-71`).
    let target_requests_farm_items = is_farmer(target) && !is_farmer(body);
    let farmer = is_farmer(body);
    if has_excess_food(body) && (farmer || target_wants_food) {
        throw_half_stack(body, store, target_position, |item_id| {
            food_points(item_id) > 0
        });
    }
    if farmer && count_item_id(body, VANILLA_ITEM_WHEAT_ID) > 32 {
        throw_half_stack(body, store, target_position, |item_id| {
            item_id == VANILLA_ITEM_WHEAT_ID
        });
    }
    if target_requests_farm_items {
        throw_half_stack(body, store, target_position, |item_id| {
            FARMER_REQUESTED_IDS.contains(&item_id)
        });
    }
}

/// `TradeWithVillager.throwHalfStack` (`TradeWithVillager.java:74`) over
/// `BehaviorUtils.throwItem` (`BehaviorUtils.java:89-105`).
fn throw_half_stack(
    body: &mut Entity,
    store: &mut EntityStore,
    towards: Vec3d,
    wanted: impl Fn(i32) -> bool,
) {
    for slot in 0..slots(body).len() {
        let Some(stack) = item(body, slot) else {
            continue;
        };
        if !stack.item_id().is_some_and(&wanted) {
            continue;
        }
        let max = stack.max_stack_size().unwrap_or(64);
        let count = if stack.count() > max / 2 {
            stack.count() / 2
        } else if stack.count() > 24 {
            stack.count() - 24
        } else {
            continue;
        };
        let mut stack = stack.clone();
        let thrown = stack.split(count);
        set_slot(body, slot, Some(stack));
        throw_item(body, store, towards, thrown);
        return;
    }
}

pub(crate) fn throw_item(
    body: &Entity,
    store: &mut EntityStore,
    towards: Vec3d,
    thrown: RuntimeItemStack,
) {
    crate::entity::item::throw_item(body, store, towards, thrown);
}

fn broadcast_event(store: &mut EntityStore, entity_id: i32, event_id: i8) {
    store.broadcast_to_tracking(
        entity_id,
        ClientboundPlayPacket::EntityEvent(EntityEvent {
            entity_id,
            event_id,
        }),
    );
}

// `VillagerMakeLove.takeVacantBed` (`VillagerMakeLove.java:92-99`).
fn take_child_bed(
    body: &Entity,
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<[i32; 3]> {
    let (malus, inherited) = store.effective_pathfinding_malus_with_provenance(body, level)?;
    let runtime = store.mob_ai(body.id())?;
    let pathing = crate::entity::ai::mob::navigation::MobPathingSnapshot::from_entity(
        body,
        runtime.has_target(),
        store.vehicle(body.id()).is_some(),
        store.difficulty(),
        malus,
    )?
    .with_inherited_malus(inherited);
    let max_path_length = (body
        .living()?
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
        as f32)
        .max(crate::entity::ai::mob::VILLAGER_REQUIRED_PATH_LENGTH);
    let home =
        bbb_facts::poi_types::vanilla_poi_type_for_resource_id(crate::entity::villager::HOME_POI)?;
    let center = body.block_position();
    let mut candidates = level.poi_mut().records_in_range(
        [center.x, center.y, center.z],
        BED_SEARCH_RANGE,
        crate::chunk::poi::PoiOccupancy::HasSpace,
        |record| record.type_id == home.resource_id,
    );
    candidates.sort_unstable_by_key(|record| {
        let [x, y, z] = record.pos;
        let dx = i64::from(x) - i64::from(center.x);
        let dy = i64::from(y) - i64::from(center.y);
        let dz = i64::from(z) - i64::from(center.z);
        dx * dx + dy * dy + dz * dz
    });
    candidates.into_iter().find_map(|record| {
        let [x, y, z] = record.pos;
        (runtime.ground_path_reaches(
            level,
            pathing,
            BlockPos { x, y, z },
            home.valid_range,
            max_path_length,
        ) == Some(true)
            && level.poi_mut().acquire_ticket(record.pos))
        .then_some(record.pos)
    })
}

pub(crate) fn villager_type_for_biome(biome: &str) -> &'static str {
    // `VillagerType.java:29-62`.
    match biome {
        "minecraft:badlands"
        | "minecraft:desert"
        | "minecraft:eroded_badlands"
        | "minecraft:wooded_badlands" => "minecraft:desert",
        "minecraft:bamboo_jungle" | "minecraft:jungle" | "minecraft:sparse_jungle" => {
            "minecraft:jungle"
        }
        "minecraft:savanna_plateau" | "minecraft:savanna" | "minecraft:windswept_savanna" => {
            "minecraft:savanna"
        }
        "minecraft:deep_frozen_ocean"
        | "minecraft:frozen_ocean"
        | "minecraft:frozen_river"
        | "minecraft:ice_spikes"
        | "minecraft:snowy_beach"
        | "minecraft:snowy_taiga"
        | "minecraft:snowy_plains"
        | "minecraft:grove"
        | "minecraft:snowy_slopes"
        | "minecraft:frozen_peaks"
        | "minecraft:jagged_peaks" => "minecraft:snow",
        "minecraft:swamp" | "minecraft:mangrove_swamp" => "minecraft:swamp",
        "minecraft:old_growth_spruce_taiga"
        | "minecraft:old_growth_pine_taiga"
        | "minecraft:windswept_gravelly_hills"
        | "minecraft:windswept_hills"
        | "minecraft:taiga"
        | "minecraft:windswept_forest" => "minecraft:taiga",
        _ => "minecraft:plains",
    }
}

pub(crate) fn give_birth(
    body: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    partner_id: i32,
    random: &crate::random::GameRandom,
) {
    let Some(partner_type) = store
        .get(partner_id)
        .filter(|partner| is_villager(partner))
        .and_then(|partner| data(partner)?.component_villager_type().map(str::to_owned))
    else {
        return;
    };
    eat_and_digest_food(body);
    if let Some(partner) = store.get_mut(partner_id) {
        eat_and_digest_food(partner);
    }
    let bed = take_child_bed(body, store, level);
    let Some(bed) = bed else {
        broadcast_event(store, partner_id, ANGRY_EVENT);
        broadcast_event(store, body.id(), ANGRY_EVENT);
        return;
    };
    // `Villager.getBreedOffspring` (`Villager.java:742-750`).
    let roll = random.next_double();
    let child_type = if roll < 0.5 {
        let biome = level.biome_resource_id(body.block_position());
        villager_type_for_biome(biome.unwrap_or("minecraft:plains")).to_owned()
    } else if roll < 0.75 {
        data(body)
            .and_then(GeneralLivingData::component_villager_type)
            .map_or(partner_type.clone(), str::to_owned)
    } else {
        partner_type
    };
    let Some(child_id) =
        store.spawn_general_living(VANILLA_ENTITY_TYPE_VILLAGER_ID, body.position())
    else {
        level.poi_mut().release_ticket(bed);
        return;
    };
    crate::entity::set_spawner_age(body, PARENT_AGE);
    if let Some(partner) = store.get_mut(partner_id) {
        crate::entity::set_spawner_age(partner, PARENT_AGE);
    }
    if let Some(child) = store.get_mut(child_id) {
        crate::entity::set_spawner_age(child, CHILD_AGE);
        crate::entity::general_living::finalize_spawner_villager_type(child, &child_type);
        child.set_rot(0.0, 0.0);
    }
    broadcast_event(store, child_id, HEART_EVENT);
    store.set_mob_brain_memory(
        child_id,
        None,
        MemoryModuleType::Home,
        Some(MemoryValue::GlobalPos(MemoryGlobalPos {
            dimension: level.dimension_key().as_str().to_owned(),
            pos: BlockPos {
                x: bed[0],
                y: bed[1],
                z: bed[2],
            },
        })),
    );
}

/// `HarvestFarmland.tick`'s work on one cell (`HarvestFarmland.java:102-125`):
/// a ripe crop is broken with its drops, an empty farmland cell takes the
/// first plantable seed from the inventory.
pub(crate) fn farm_at(entity: &mut Entity, level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    let state = level.block_state(pos);
    if crate::block_behavior::crop::is_crop(state) && crate::block_behavior::crop::is_max_age(state)
    {
        level.destroy_block(pos, true);
    }
    let below = level.block_state(BlockPos {
        y: pos.y - 1,
        ..pos
    });
    if !bbb_server_types::fluid::block_state_is_air(state) || !is_farmland(below) {
        return;
    }
    for slot in 0..slots(entity).len() {
        let Some(stack) = item(entity, slot) else {
            continue;
        };
        let (Some(item_id), Some(item)) = (stack.item_id(), stack.item_resource_id()) else {
            continue;
        };
        if !level.registry_tag_membership(
            "minecraft:item",
            "minecraft:villager_plantable_seeds",
            item,
        ) {
            continue;
        }
        let Some(place) = seed_block_state(item_id) else {
            continue;
        };
        level.set_block(
            pos,
            place,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
            crate::neighbor_updater::UPDATE_LIMIT,
        );
        level.game_event_with_state(
            crate::level_event::VanillaGameEvent::BLOCK_PLACE,
            pos,
            Some(entity.id()),
            Some(place),
        );
        level.play_block_sound(pos, "minecraft:item.crop.plant");
        let mut stack = stack.clone();
        stack.shrink(1);
        set_slot(entity, slot, Some(stack));
        break;
    }
}

const BONE_MEAL: &str = "minecraft:bone_meal";

pub(crate) fn has_bone_meal(entity: &Entity) -> bool {
    data(entity)
        .and_then(GeneralLivingData::internal_inventory)
        .is_some_and(|inventory| {
            inventory.original_items().any(|stack| {
                stack.count() > 0 && stack.item_id() == Some(VANILLA_ITEM_BONE_MEAL_ID)
            })
        })
}

pub(crate) fn has_farm_seeds(entity: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    data(entity)
        .and_then(GeneralLivingData::internal_inventory)
        .is_some_and(|inventory| {
            inventory.original_items().any(|stack| {
                stack.count() > 0
                    && stack.item_resource_id().is_some_and(|item| {
                        level.registry_tag_membership(
                            "minecraft:item",
                            "minecraft:villager_plantable_seeds",
                            item,
                        )
                    })
            })
        })
}

/// `UseBonemeal.tick`'s growth with the first bone meal stack
/// (`UseBonemeal.java:97-121`).
pub(crate) fn bonemeal_at(entity: &mut Entity, level: &mut LevelCtx<'_, '_>, pos: BlockPos) {
    if !has_bone_meal(entity) || !crate::block_behavior::bonemeal::grow_crop(level, pos) {
        return;
    }
    remove_item_type(entity, VANILLA_ITEM_BONE_MEAL_ID, 1);
    level.level_event(1505, pos, 15);
}

/// The main-hand writes of `UseBonemeal.start`/`stop` and
/// `ShowTradesToPlayer.displayAsHeldItem`/`clearHeldItem`
/// (`ShowTradesToPlayer.java:104-112`).
pub(crate) fn set_held_item(entity: &mut Entity, held: crate::entity::ai::VillagerHeldItem) {
    use crate::entity::ai::VillagerHeldItem;
    use bbb_protocol::equipment_slots::EquipmentSlot;
    let (stack, drop_chance) = match held {
        VillagerHeldItem::Clear => (None, Some(0.085)),
        VillagerHeldItem::BoneMeal => (crate::entity::item::plain_stack(BONE_MEAL, 1), None),
        VillagerHeldItem::Trade { item_id, index } => {
            let item =
                bbb_protocol::item_types::vanilla_item_type(item_id).map(|item| item.resource_id);
            let shown: Vec<&RuntimeItemStack> = entity
                .merchant_offers()
                .map(|offers| {
                    offers
                        .offers()
                        .iter()
                        .filter(|offer| {
                            !offer.is_out_of_stock()
                                && (Some(offer.base_cost_a().item_resource_id()) == item
                                    || offer.cost_b().map(|cost| cost.item_resource_id()) == item)
                        })
                        .map(|offer| offer.result())
                        .collect()
                })
                .unwrap_or_default();
            match shown.len() {
                0 => (None, Some(0.085)),
                len => (Some(shown[index % len].clone()), Some(0.0)),
            }
        }
    };
    let Some(living) = entity.living_mut() else {
        return;
    };
    let _ = living.set_item_slot(EquipmentSlot::MainHand, stack);
    if let (Some(chance), EntityData::GeneralLiving(data)) = (drop_chance, &mut entity.data) {
        if let Some(mob) = data.mob.as_mut() {
            let _ = mob.set_drop_chance(EquipmentSlot::MainHand, chance);
        }
    }
}

fn is_farmland(state: i32) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_FARMLAND_ID)
}

/// `BlockItem.getBlock().defaultBlockState()` of a seed item.
fn seed_block_state(item_id: i32) -> Option<i32> {
    let block_id = match item_id {
        VANILLA_ITEM_WHEAT_SEEDS_ID => VANILLA_BLOCK_WHEAT_ID,
        VANILLA_ITEM_BEETROOT_SEEDS_ID => VANILLA_BLOCK_BEETROOTS_ID,
        VANILLA_ITEM_CARROT_ID => VANILLA_BLOCK_CARROTS_ID,
        VANILLA_ITEM_POTATO_ID => VANILLA_BLOCK_POTATOES_ID,
        VANILLA_ITEM_TORCHFLOWER_SEEDS_ID => VANILLA_BLOCK_TORCHFLOWER_CROP_ID,
        VANILLA_ITEM_PITCHER_POD_ID => VANILLA_BLOCK_PITCHER_CROP_ID,
        _ => return None,
    };
    bbb_protocol::block_state_types::vanilla_block_default_state_id(block_id)
}

/// `WorkAtComposter.useWorkstation` (`WorkAtComposter.java:23-33`): bake
/// bread from wheat, then compost spare seeds.
pub(crate) fn use_composter(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
) {
    let mut state = level.block_state(pos);
    if !crate::block_behavior::composter::is_composter(state) {
        return;
    }
    // `makeBread` (`:80-96`).
    if count_item_id(entity, VANILLA_ITEM_BREAD_ID) <= 36 {
        let bread = (count_item_id(entity, VANILLA_ITEM_WHEAT_ID) / 3).min(3);
        if bread != 0 {
            remove_item_type(entity, VANILLA_ITEM_WHEAT_ID, bread * 3);
            if let Some(stack) = crate::entity::item::plain_stack("minecraft:bread", bread) {
                let rest = add_item(entity, level, stack);
                if !rest.is_empty() {
                    let position = entity.position();
                    crate::entity::item::spawn_dropped_item_entity(
                        store,
                        Vec3d {
                            y: position.y + 0.5,
                            ..position
                        },
                        rest,
                    );
                }
            }
        }
    }
    // `compostItems` (`:35-71`).
    let before = state;
    if crate::block_behavior::composter::fill_level(state) == 8 {
        state = extract_produce(store, level, pos, state, entity.id());
    }
    let mut budget = 20;
    let mut seen = [0; 2];
    'slots: for slot in (0..slots(entity).len()).rev() {
        let Some(stack) = item(entity, slot) else {
            continue;
        };
        let index = match stack.item_id() {
            Some(VANILLA_ITEM_WHEAT_SEEDS_ID) => 0,
            Some(VANILLA_ITEM_BEETROOT_SEEDS_ID) => 1,
            _ => continue,
        };
        seen[index] += stack.count();
        let use_count = (seen[index] - 10).min(budget).min(stack.count());
        if use_count <= 0 {
            continue;
        }
        budget -= use_count;
        let mut stack = stack.clone();
        for _ in 0..use_count {
            // `ComposterBlock.insertItem` only takes items below level 7.
            if crate::block_behavior::composter::fill_level(state) >= 7 {
                set_slot(entity, slot, Some(stack));
                break 'slots;
            }
            let item_id = stack.item_id().unwrap_or_default();
            stack.shrink(1);
            state = crate::block_behavior::composter::add_item(
                level,
                pos,
                state,
                item_id,
                Some(entity.id()),
            );
            if crate::block_behavior::composter::fill_level(state) == 7 {
                set_slot(entity, slot, Some(stack));
                break 'slots;
            }
        }
        set_slot(entity, slot, Some(stack));
    }
    level.level_event(1500, pos, i32::from(state != before));
}

/// `ComposterBlock.extractProduce` (`ComposterBlock.java:298-309`).
fn extract_produce(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    state: i32,
    entity_id: i32,
) -> i32 {
    let random = store.random().clone();
    let offset = |random: &crate::random::GameRandom| (f64::from(random.next_float()) - 0.5) * 0.7;
    let position = Vec3d {
        x: f64::from(pos.x) + 0.5 + offset(&random),
        y: f64::from(pos.y) + 1.01,
        z: f64::from(pos.z) + 0.5 + offset(&random),
    };
    if let Some(bone_meal) = crate::entity::item::plain_stack("minecraft:bone_meal", 1) {
        crate::entity::item::spawn_dropped_item_entity(store, position, bone_meal);
    }
    let emptied = crate::block_behavior::composter::empty(level, pos, state, Some(entity_id));
    level.play_block_sound(pos, "minecraft:block.composter.empty");
    emptied
}

/// `Villager.setUnhappy` (`Villager.java:328-333`) without its sound, which
/// the refusal plays itself.
pub(crate) fn set_unhappy(entity: &mut Entity) {
    set_unhappy_counter(entity, UNHAPPY_TICKS);
}

fn set_unhappy_counter(entity: &mut Entity, value: i32) {
    let Some(merchant) =
        data_mut(entity).and_then(GeneralLivingData::abstract_villager_retained_mut)
    else {
        return;
    };
    if merchant.unhappy_counter != value {
        merchant.unhappy_counter = value;
        entity.mark_synched_dirty(VANILLA_ABSTRACT_VILLAGER_UNHAPPY_COUNTER_DATA_ID);
    }
}

/// `Villager.tick`'s unhappy countdown and `maybeDecayGossip`
/// (`Villager.java:287-294`, `:824-832`).
pub(crate) fn tick(entity: &mut Entity, game_time: i64) {
    let counter = data(entity)
        .and_then(GeneralLivingData::abstract_villager_retained)
        .map_or(0, |merchant| merchant.unhappy_counter);
    if counter > 0 {
        set_unhappy_counter(entity, counter - 1);
    }
    let Some(last_decay) = data(entity)
        .and_then(GeneralLivingData::villager_work)
        .map(|work| work.last_gossip_decay)
    else {
        return;
    };
    if last_decay != 0 && game_time < last_decay.saturating_add(GOSSIP_DECAY_INTERVAL) {
        return;
    }
    if let Some(work) = data_mut(entity).and_then(GeneralLivingData::villager_work_mut) {
        work.last_gossip_decay = game_time;
    }
    if last_decay != 0 {
        if let Some(gossips) = data_mut(entity).and_then(GeneralLivingData::villager_gossips_mut) {
            gossips.retain_mut(|gossip| {
                gossip.value = gossip.value.saturating_sub(gossip.kind.decay_per_day());
                gossip.value >= GOSSIP_DISCARD_THRESHOLD
            });
        }
    }
}

/// `Villager.setUnhappy`'s 40 ticks.
const UNHAPPY_TICKS: i32 = 40;
/// `Villager.maybeDecayGossip`'s day (`Villager.java:828`).
const GOSSIP_DECAY_INTERVAL: i64 = 24_000;
/// `Villager.gossip`'s cooldown (`Villager.java:815-816`).
const GOSSIP_INTERVAL: i64 = 1200;
/// `GossipContainer.DISCARD_THRESHOLD` (`GossipContainer.java:32`).
const GOSSIP_DISCARD_THRESHOLD: i32 = 2;

fn last_gossip_time(entity: &Entity) -> i64 {
    data(entity)
        .and_then(GeneralLivingData::villager_work)
        .map_or(0, |work| work.last_gossip_time)
}

fn set_last_gossip_time(entity: &mut Entity, timestamp: i64) {
    if let Some(work) = data_mut(entity).and_then(GeneralLivingData::villager_work_mut) {
        work.last_gossip_time = timestamp;
    }
}

/// `Villager.gossip` (`Villager.java:814-822`) over
/// `GossipContainer.transferFrom` (`GossipContainer.java:98-106`): the body
/// takes up to ten of the target's gossips, weighted by their worth. Answers
/// whether they gossiped, which calls a golem.
pub(crate) fn gossip(
    body: &mut Entity,
    store: &mut EntityStore,
    target_id: i32,
    timestamp: i64,
    random: &crate::random::GameRandom,
) -> bool {
    let ready = |last: i64| timestamp < last || timestamp >= last.saturating_add(GOSSIP_INTERVAL);
    let Some(target) = store
        .get_mut(target_id)
        .filter(|target| is_villager(target))
    else {
        return false;
    };
    if !ready(last_gossip_time(body)) || !ready(last_gossip_time(target)) {
        return false;
    }
    set_last_gossip_time(target, timestamp);
    let source = data(target)
        .and_then(GeneralLivingData::villager_gossips)
        .map(ToOwned::to_owned)
        .unwrap_or_default();
    set_last_gossip_time(body, timestamp);
    // `selectGossipsForTransfer` (`GossipContainer.java:68-91`).
    let mut ranges = Vec::with_capacity(source.len());
    let mut end = 0i32;
    for gossip in &source {
        end = end.saturating_add(
            gossip
                .value
                .saturating_mul(gossip.kind.weight())
                .saturating_abs(),
        );
        ranges.push(end.saturating_sub(1));
    }
    if end <= 0 {
        return true;
    }
    let mut chosen = vec![false; source.len()];
    for _ in 0..10 {
        let choice = random.next_int(end);
        if let Some(index) = ranges.iter().position(|range| *range >= choice) {
            chosen[index] = true;
        }
    }
    if let Some(own) = data_mut(body).and_then(GeneralLivingData::villager_gossips_mut) {
        for (gossip, _) in source.iter().zip(&chosen).filter(|(_, chosen)| **chosen) {
            let value = gossip
                .value
                .saturating_sub(gossip.kind.decay_per_transfer());
            if value < GOSSIP_DISCARD_THRESHOLD {
                continue;
            }
            match own
                .iter_mut()
                .find(|mine| mine.target == gossip.target && mine.kind == gossip.kind)
            {
                Some(mine) => mine.value = mine.value.max(value),
                None => {
                    own.push(crate::entity::general_living::VillagerGossip { value, ..*gossip })
                }
            }
        }
    }
    true
}
