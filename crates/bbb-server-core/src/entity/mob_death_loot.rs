use crate::world::simulation_level::LevelCtx;
use std::collections::BTreeMap;

use crate::block_behavior::vegetation::{may_place_on, VegetationSupport};
use crate::loot_runtime::{
    LootEntities, LootEntity, LootEntityAnswer, LootEntityFacts, LootEntityFlags,
    LootEntityPosition, LootOrigin, LootRaiderFlags, LootTool, StoredLootEntity,
};
use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_SPRINTING, VANILLA_ENTITY_SHARED_FLAG_SWIMMING,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_FOX_ID,
    VANILLA_ENTITY_TYPE_PIGLIN_ID, VANILLA_ENTITY_TYPE_WITHER_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::ItemStackSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::damage::{StoredDamageCausingEntity, StoredEntityDamageSource};
use crate::entity::store::{EntityBodyReadView, EntityStore};
use crate::entity::{Entity, EntityData};

/// `Entity.getSharedFlag(7)` (`Entity.java:2712-2714`), which
/// `LivingEntity.isFallFlying` reads (`LivingEntity.java:3588-3590`).
const SHARED_FLAG_FALL_FLYING: i8 = 1 << 7;
/// `Pose.CROUCHING` (`Pose.java`), which `Entity.isCrouching` tests
/// (`Entity.java:2631-2633`).
const CROUCHING_POSE_ID: i32 = bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_CROUCHING_ID;

/// `Items.WITHER_ROSE` (`Items.java`).
const WITHER_ROSE_ITEM: &str = "minecraft:wither_rose";
/// `Blocks.WITHER_ROSE`'s default state, the one `createWitherRose` builds
/// (`LivingEntity.java:1465`).
const WITHER_ROSE_BLOCK: &str = "minecraft:wither_rose";
/// `EntityType.WITHER`'s built-in type id, the `killer instanceof WitherBoss`
/// test (`LivingEntity.java:1462`).
const WITHER_ENTITY_TYPE_ID: i32 = bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID;
/// `Block.UPDATE_ALL` (`Block.java:104`), the flag `createWitherRose` sets its
/// rose with (`LivingEntity.java:1467`).
const SET_BLOCK_UPDATE_ALL: i32 = 3;

/// What a death loot table asks the level that owns the loot registries for.
///
pub struct MobLootRequest<'a> {
    pub table_id: &'a str,
    pub origin: LootOrigin,
    pub entities: LootEntities<'a>,
    pub attacking_entity: MobLootAttacker<'a>,
    /// `LootContextParams.DIRECT_ATTACKING_ENTITY` (`:1547`).
    pub direct_attacking_entity: MobLootAttacker<'a>,
    pub damage_source: MobLootDamageSource<'a>,
    ///
    /// `LootParams.getLuck` and `LAST_DAMAGE_PLAYER` both come from that
    /// player, and the luck is its `minecraft:luck` attribute, which lives on
    /// the session rather than on a stored entity: the level that owns the
    /// sessions resolves this id, which is also what
    /// `EntityReference.getPlayer(level)` does.
    pub killer_player: Option<Uuid>,
    pub seed: i64,
}

pub struct EquipmentDropChanceRequest<'a> {
    pub bodies: crate::entity::store::EntityBodyReadView<'a>,
    /// The explicit living subject of `processEquipmentDropChance`.
    pub subject: MobLootAttacker<'a>,
    pub original_chance: f32,
    pub attacker: MobLootAttacker<'a>,
    /// `killingBlow.getEntity() instanceof LivingEntity`, which opens the
    /// attacker pass.
    pub attacker_living: bool,
    pub direct_attacking_entity: MobLootAttacker<'a>,
    pub damage_source: MobLootDamageSource<'a>,
}

/// `LootContextParams.DAMAGE_SOURCE` as far as this crate holds it.
///
#[derive(Debug, Clone, Copy)]
pub struct MobLootDamageSource<'a> {
    /// The damage type's resource id, e.g. `minecraft:player_attack`.
    pub damage_type: &'a str,
    /// `DamageSource.isDirect()`.
    pub is_direct: bool,
}

impl MobLootDamageSource<'_> {
    pub(crate) fn of<'a>(source: &'a StoredEntityDamageSource<'a>) -> MobLootDamageSource<'a> {
        let causing = source
            .causing_entity
            .map(StoredDamageCausingEntity::entity_id);
        MobLootDamageSource {
            damage_type: &source.damage_type.resource_id,
            is_direct: match (causing, source.direct_entity_id) {
                (None, None) => true,
                (Some(causing), Some(direct)) => causing == direct,
                _ => false,
            },
        }
    }
}

/// One of the two attacker slots before the level has bound it.
///
#[derive(Clone, Copy, Default)]
pub enum MobLootAttacker<'a> {
    #[default]
    Absent,
    /// A readable entity.
    Entity(LootEntity<'a>),
    /// A stored entity the store no longer holds.
    Opaque,
    Stored {
        entity_id: i32,
    },
    /// A session player, which only the level can show.
    SessionPlayer {
        uuid: Uuid,
    },
}

impl std::fmt::Debug for MobLootAttacker<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Absent => "Absent",
            Self::Entity(_) => "Entity",
            Self::Opaque => "Opaque",
            Self::Stored { .. } => "Stored",
            Self::SessionPlayer { .. } => "SessionPlayer",
        })
    }
}

impl<'a> MobLootAttacker<'a> {
    /// What `LootEntities` carries for this slot.
    #[must_use]
    pub fn to_loot_entity(&self) -> LootEntity<'a> {
        match self {
            Self::Absent => LootEntity::Absent,
            Self::Entity(entity) => *entity,
            Self::Opaque | Self::Stored { .. } | Self::SessionPlayer { .. } => LootEntity::Opaque,
        }
    }
}

/// What one death loot table drew.
#[derive(Debug, Clone, PartialEq)]
pub enum MobLootRoll {
    /// `LootTable.getRandomItems`' stacks, in draw order.
    Drawn(Vec<crate::loot_runtime::LootStack>),
    /// The level holds no loot registry at all, which is not a table with no
    /// entry: the death cannot be drawn and says so.
    NoRegistries,
    /// The loot engine refused a node in the table, naming it.
    Refused(String),
}

pub(crate) fn drop_all_death_loot(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &mut Entity,
    source: StoredEntityDamageSource<'_>,
) -> Option<()> {
    // `Fox.dropAllDeathLoot` (`Fox.java:700-707`) preserves the whole mouth stack.
    if victim.entity_type_id() == VANILLA_ENTITY_TYPE_FOX_ID
        && spawn_equipment_stack(level, victim, EquipmentSlot::MainHand)?
    {
        crate::entity::item_slots::set_living_item_in_tick(
            victim,
            store,
            level,
            EquipmentSlot::MainHand,
            None,
        )
        .ok()?;
    }
    let player_killed = victim
        .living()
        .is_some_and(|living| living.last_hurt_by_player_memory_time > 0);
    if should_drop_loot(victim, level) {
        drop_from_loot_table(store, level, victim, source, player_killed);
        drop_custom_death_loot(store, level, victim, source, player_killed)?;
    }
    drop_equipment(store, level, victim)?;
    let killer = source
        .causing_entity
        .map(crate::entity::damage::StoredDamageCausingEntity::entity_id);
    drop_experience(store, level, victim, player_killed, killer);
    create_wither_rose(store, level, victim);
    Some(())
}

///
/// Each override runs `super.dropEquipment` first, so a chested horse reaches
/// `AbstractHorse`'s body before its own chest statement.
fn drop_equipment(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &mut Entity,
) -> Option<()> {
    match victim.entity_type_id() {
        VANILLA_ENTITY_TYPE_ALLAY_ID => {
            drop_allay_inventory(level, victim);
            drop_hand_item_unless_enchantment_holds_it(store, level, victim)?;
        }
        VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID => {
            drop_preserved_equipment(store, level, victim, false)?
        }
        _ => {}
    }
    drop_mount_inventory(store, level, victim);
    drop_chest(level, victim);
    Some(())
}

fn drop_allay_inventory(level: &mut LevelCtx<'_, '_>, victim: &mut Entity) {
    let position = victim.position();
    let EntityData::GeneralLiving(data) = &mut victim.data else {
        return;
    };
    let Some(inventory) = data.internal_inventory_mut() else {
        return;
    };
    let removed = inventory.remove_all_items();
    for (slot, stack) in removed {
        if !spawn_internal_inventory_stack(level, position, stack.clone()) {
            inventory.set_item(slot, Some(stack));
        }
    }
}

fn drop_mount_inventory(store: &EntityStore, level: &mut LevelCtx<'_, '_>, victim: &Entity) {
    let EntityData::GeneralLiving(data) = victim.data() else {
        return;
    };
    if data.abstract_horse_retained().is_none() {
        return;
    }
    let Some(inventory) = data.internal_inventory() else {
        return;
    };
    for stack in inventory.original_items() {
        let held = Some(stack);
        if held.as_ref().is_none_or(|value| value.is_empty()) {
            continue;
        }
        let prevented =
            crate::entity::armor_stand::stack_enchantment_ids(held.as_ref().unwrap(), store)
                .iter()
                .any(|id| store.prevents_equipment_drop(id));
        if !prevented {
            spawn_internal_inventory_stack(level, victim.position(), stack.clone());
        }
    }
}

fn spawn_internal_inventory_stack(
    level: &mut LevelCtx<'_, '_>,
    position: bbb_protocol::packets::Vec3d,
    stack: bbb_server_types::item_stack::RuntimeItemStack,
) -> bool {
    level.spawn_runtime_item(position, stack, true).is_some()
}

fn drop_hand_item_unless_enchantment_holds_it(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &mut Entity,
) -> Option<()> {
    let slot = EquipmentSlot::MainHand;
    let Some(stack) = victim.living().and_then(|living| living.item_by_slot(slot)) else {
        return Some(());
    };
    if stack.is_empty()
        || crate::entity::armor_stand::stack_enchantment_ids(stack, store)
            .iter()
            .any(|id| store.prevents_equipment_drop(id))
    {
        return Some(());
    }
    if spawn_equipment_stack(level, victim, slot)? {
        crate::entity::item_slots::set_living_item_in_tick(victim, store, level, slot, None)
            .ok()?;
    }
    Some(())
}

/// `Mob.dropPreservedEquipment` (`Mob.java:897-913`); `keep_bound` is the
/// cure's predicate that leaves `PREVENT_ARMOR_CHANGE` gear on the body.
pub(crate) fn drop_preserved_equipment(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &mut Entity,
    keep_bound: bool,
) -> Option<()> {
    for entry in bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS {
        let slot = entry.slot;
        let EntityData::GeneralLiving(data) = victim.data() else {
            return Some(());
        };
        let Some(mob) = data.mob_retained() else {
            return Some(());
        };
        if mob.drop_chance(slot) <= 1.0 {
            continue;
        }
        let Some(original) = victim.living()?.equipment().get(slot).cloned() else {
            continue;
        };
        if keep_bound && prevents_armor_change(&original, store) {
            continue;
        }
        crate::entity::item_slots::set_living_item_in_tick(victim, store, level, slot, None)
            .ok()?;
        level.spawn_runtime_item(victim.position(), original, true)?;
    }
    Some(())
}

pub(crate) fn prevents_armor_change(stack: &RuntimeItemStack, store: &EntityStore) -> bool {
    crate::entity::armor_stand::stack_enchantment_ids(stack, store)
        .iter()
        .any(|id| store.prevents_armor_change(id))
}

/// `ZombieVillager.finishConversion` (`ZombieVillager.java:239-247`): the gear
/// the cure kept goes into the villager's inventory slot `index`.
pub(crate) fn move_bound_gear_to_inventory(
    source: &Entity,
    target: &mut Entity,
    store: &EntityStore,
) {
    let Some(equipment) = source.living().map(|living| living.equipment()) else {
        return;
    };
    let EntityData::GeneralLiving(data) = &mut target.data else {
        return;
    };
    let Some(inventory) = data.internal_inventory_mut() else {
        return;
    };
    for entry in bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS {
        if let Some(stack) = equipment
            .get(entry.slot)
            .filter(|stack| prevents_armor_change(stack, store))
        {
            inventory.set_item(entry.index as usize, Some(stack.clone()));
        }
    }
}

fn drop_chest(level: &mut LevelCtx<'_, '_>, victim: &mut Entity) {
    let EntityData::GeneralLiving(data) = victim.data() else {
        return;
    };
    let cleared = match data.horse_phenotype() {
        Some(crate::entity::horse_phenotype::HorsePhenotypeState::Chested { has_chest: true }) => {
            crate::entity::horse_phenotype::HorsePhenotypeState::chested(false)
        }
        Some(crate::entity::horse_phenotype::HorsePhenotypeState::Llama {
            has_chest: true,
            strength,
            variant,
        }) => crate::entity::horse_phenotype::HorsePhenotypeState::llama(false, strength, variant),
        _ => return,
    };
    let stack = ItemStackSummary {
        item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_CHEST_ID),
        count: 1,
        component_patch: Default::default(),
    };
    level.spawn_item(victim.position(), &stack);
    crate::entity::horse_phenotype::set_state(victim, cleared);
}

fn spawn_equipment_stack(
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
    slot: EquipmentSlot,
) -> Option<bool> {
    let Some(original) = victim
        .living()
        .and_then(|living| living.equipment().get(slot).cloned())
    else {
        return Some(false);
    };
    level.spawn_runtime_item(victim.position(), original, true)?;
    Some(true)
}

///
/// ```java
/// if (!this.wasExperienceConsumed()
///       && (this.isAlwaysExperienceDropper()
///          || this.lastHurtByPlayerMemoryTime > 0 && this.shouldDropExperience()
///             && level.getGameRules().get(GameRules.MOB_DROPS))) {
///    ExperienceOrb.award(level, this.position(), this.getExperienceReward(level, killer));
/// }
/// ```
///
fn drop_experience(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
    player_killed: bool,
    killer: Option<i32>,
) {
    let Some(living) = victim.living() else {
        return;
    };
    if living.was_experience_consumed() {
        return;
    }
    if !player_killed
        || !crate::entity::experience::should_drop_experience_with_registry(
            victim,
            store.plugins().content_registry(),
        )
        || !level.mob_drops()
    {
        return;
    }
    let position = victim.position();
    // `LivingEntity.die` raised `ENTITY_DIE` immediately before this call, and
    // a listener may still run and consume the reward before the orbs are paid;
    // the level posts the queued award once that frame's events are delivered.
    level.queue_death_experience(store, victim, position, killer);
}

fn should_drop_loot(victim: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    if !level.mob_drops() {
        return false;
    }
    let EntityData::GeneralLiving(data) = victim.data() else {
        return true;
    };
    if !data.is_baby() {
        return true;
    }
    // `Monster.shouldDropLoot`, `Hoglin.shouldDropLoot`, `Zoglin.shouldDropLoot`:
    // baby monsters/hoglins/zoglins still drop loot and picked-up equipment.
    let resource_id = data.entity_resource_id();
    if matches!(resource_id, "minecraft:hoglin" | "minecraft:zoglin") {
        return true;
    }
    let mob_category = bbb_protocol::entity_types::vanilla_entity_type(victim.entity_type_id())
        .map(|facts| facts.mob_category)
        .or_else(|| {
            level
                .content_registry()
                .entity_by_id(victim.entity_type_id())
                .map(|spec| spec.category)
        });
    resource_id != "minecraft:zombie_horse"
        && mob_category == Some(bbb_protocol::entity_types::VanillaEntityMobCategory::Monster)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn entity_loot_table(entity: &Entity) -> Option<&str> {
    entity_loot_table_with_registry(
        entity,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn entity_loot_table_with_registry<'a>(
    entity: &'a Entity,
    content_registry: &'a bbb_protocol::content_registry::ContentRegistry,
) -> Option<&'a str> {
    let custom = match entity.data() {
        EntityData::GeneralLiving(data) => {
            data.mob_retained().and_then(|mob| mob.death_loot_table())
        }
        _ => None,
    };
    custom
        .or_else(|| {
            bbb_protocol::entity_types::vanilla_entity_type(entity.entity_type_id())
                .and_then(|kind| kind.default_loot_table)
        })
        .or_else(|| {
            content_registry
                .entity_by_id(entity.entity_type_id())
                .and_then(|spec| spec.default_loot_table.as_deref())
        })
}

fn drop_from_loot_table(
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    player_killed: bool,
) {
    let mob = match victim.data() {
        EntityData::GeneralLiving(data) => data.mob_retained(),
        _ => None,
    };
    let seed = mob.map_or(0, |mob| mob.death_loot_table_seed());
    let Some(table_id) =
        entity_loot_table_with_registry(victim, store.plugins().content_registry())
            .map(str::to_owned)
    else {
        return;
    };
    roll_loot_table(store, level, victim, source, player_killed, &table_id, seed);
    if let EntityData::GeneralLiving(data) = &mut victim.data {
        if let Some(mob) = data.mob_retained_mut() {
            mob.set_death_loot_table(None);
        }
    }
}

/// `Creeper.killedEntity` (Creeper.java:171-180): a charged creeper's blast
/// drops the first head it earns.
pub(crate) fn drop_charged_creeper_loot(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    killer_id: i32,
    victim: &Entity,
    source: StoredEntityDamageSource<'_>,
) {
    if store.is_charged_creeper_blast(killer_id)
        && level.mob_drops()
        && roll_loot_table(
            store,
            level,
            victim,
            source,
            false,
            "minecraft:charged_creeper/root",
            0,
        )
    {
        store.spend_charged_creeper_blast(killer_id);
    }
}

/// `LivingEntity.dropFromLootTable` (LivingEntity.java:1529-1555); `true`
/// when the table spawned any stack.
fn roll_loot_table(
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
    source: StoredEntityDamageSource<'_>,
    player_killed: bool,
    table_id: &str,
    seed: i64,
) -> bool {
    let position = victim.position();
    let slots = attacker_slots(&source);
    let killer_player = player_killed
        .then(|| {
            victim
                .living()
                .and_then(|living| living.last_hurt_by_player)
        })
        .flatten();
    let entities = LootEntities {
        this_entity: stored_loot_entity(store, level, victim),
        last_damage_player: if killer_player.is_some() {
            LootEntity::Opaque
        } else {
            LootEntity::Absent
        },
        ..LootEntities::default()
    };
    let stored = |slot| match slot {
        AttackerSlot::Stored(id) => store.get(id),
        _ => None,
    };
    let attacking_entity = attacker_slot(
        slots[0],
        stored(slots[0]).map(|entity| stored_loot_entity(store, level, entity)),
        source.attacker_views.causing,
    );
    let direct_attacking_entity = attacker_slot(
        slots[1],
        stored(slots[1]).map(|entity| stored_loot_entity(store, level, entity)),
        source.attacker_views.direct,
    );
    let request = MobLootRequest {
        table_id,
        origin: LootOrigin {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        entities,
        attacking_entity,
        direct_attacking_entity,
        damage_source: MobLootDamageSource::of(&source),
        killer_player,
        seed,
    };
    let roll = level.mob_loot_drops(
        Some(EntityBodyReadView::new(store, Some(victim), None)),
        request,
    );
    let mut spawned = false;
    if let MobLootRoll::Drawn(stacks) = roll {
        for stack in stacks {
            let stack = RuntimeItemStack::from_summary(stack);
            if !stack.is_empty() {
                let _ = level.spawn_runtime_item(position, stack, true);
                spawned = true;
            }
        }
    }
    spawned
}

fn attacker_slots(source: &StoredEntityDamageSource<'_>) -> [AttackerSlot; 2] {
    let causing = match source.causing_entity {
        None => AttackerSlot::None,
        Some(StoredDamageCausingEntity::Player { uuid, .. }) => AttackerSlot::Player(uuid),
        Some(StoredDamageCausingEntity::Mob { entity_id })
        | Some(StoredDamageCausingEntity::Other { entity_id }) => AttackerSlot::Stored(entity_id),
    };
    let direct = match source.direct_entity_id {
        None => AttackerSlot::None,
        // `DamageSource.isDirect()`: a melee blow's direct entity *is* the
        // causing one, so the slot takes the same answer rather than looking
        // the id up again.
        Some(id) if source.causing_entity.is_some_and(|c| c.entity_id() == id) => causing,
        Some(id) => AttackerSlot::Stored(id),
    };
    [causing, direct]
}

#[derive(Clone, Copy)]
enum AttackerSlot {
    None,
    Player(Uuid),
    Stored(i32),
}

/// One slot's entity: the stored one this crate resolved, the session player
/// the level lent, or the id the level still has to bind.
fn attacker_slot<'a>(
    slot: AttackerSlot,
    stored: Option<LootEntity<'a>>,
    level_view: Option<&'a LootEntityFacts>,
) -> MobLootAttacker<'a> {
    match slot {
        AttackerSlot::None => MobLootAttacker::Absent,
        AttackerSlot::Player(uuid) => match level_view {
            Some(facts) => MobLootAttacker::Entity(LootEntity::Facts(facts)),
            None => MobLootAttacker::SessionPlayer { uuid },
        },
        AttackerSlot::Stored(entity_id) => match stored {
            Some(entity) => MobLootAttacker::Entity(entity),
            None => MobLootAttacker::Stored { entity_id },
        },
    }
}

/// A live entity with the level's raid answer at its position.
pub(crate) fn stored_loot_entity<'a, 'l: 'a>(
    store: &'a EntityStore,
    level: &LevelCtx<'l, '_>,
    entity: &'a Entity,
) -> LootEntity<'a> {
    LootEntity::Stored(StoredLootEntity {
        entity,
        bodies: Some(EntityBodyReadView::new(
            store,
            Some(entity),
            level.detached_bodies(),
        )),
        raid_at_position: level.raid_at(entity.block_position()),
    })
}

/// Binds an attacker the entity code named by id to the body the level holds.
pub(crate) fn bind_body_attacker<'a>(
    bodies: EntityBodyReadView<'a>,
    level: &LevelCtx<'_, '_>,
    attacker: MobLootAttacker<'a>,
) -> MobLootAttacker<'a> {
    match attacker {
        MobLootAttacker::Stored { entity_id } => bodies.get(entity_id).map_or(attacker, |entity| {
            MobLootAttacker::Entity(LootEntity::Stored(StoredLootEntity {
                entity,
                bodies: Some(bodies),
                raid_at_position: level.raid_at(entity.block_position()),
            }))
        }),
        other => other,
    }
}

pub(crate) fn loot_entity_position(entity: &Entity) -> LootEntityPosition {
    let position = entity.position();
    LootEntityPosition {
        x: position.x,
        y: position.y,
        z: position.z,
    }
}

pub(crate) fn loot_entity_flags(entity: &Entity, on_fire: Option<bool>) -> LootEntityFlags {
    let shared = entity.shared_flags();
    LootEntityFlags {
        on_ground: Some(entity.on_ground()),
        on_fire,
        crouching: Some(entity.pose_id() == CROUCHING_POSE_ID),
        sprinting: Some(shared & VANILLA_ENTITY_SHARED_FLAG_SPRINTING != 0),
        swimming: Some(shared & VANILLA_ENTITY_SHARED_FLAG_SWIMMING != 0),
        in_water: Some(entity.was_touching_water()),
        fall_flying: Some(shared & SHARED_FLAG_FALL_FLYING != 0),
        // `Player.getAbilities().flying` is a player field; no store holds
        // a player, so a stored entity never has it.
        ability_flying: Some(false),
        baby: Some(match entity.data() {
            EntityData::GeneralLiving(data) => data.is_baby(),
            _ => false,
        }),
    }
}

pub(crate) fn loot_slime_size(entity: &Entity) -> LootEntityAnswer<i32> {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return LootEntityAnswer::NotApplicable;
    };
    match data.slime_retained() {
        Some(slime) => LootEntityAnswer::Value(slime.size()),
        None => LootEntityAnswer::NotApplicable,
    }
}

pub(crate) fn loot_raider_flags(
    entity: &Entity,
    raid_at_position: Option<bool>,
    items: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> LootEntityAnswer<LootRaiderFlags> {
    if !crate::entity::raider::is_raider(entity.entity_type_id()) {
        return LootEntityAnswer::NotApplicable;
    }
    LootEntityAnswer::Value(LootRaiderFlags {
        has_raid: crate::entity::raider::has_raid(entity, raid_at_position),
        is_captain: crate::entity::raider::is_captain(entity, items),
    })
}

/// `LivingEntity.getItemBySlot`'s stack as the read surface `ItemPredicate`
/// asks of it: `ItemStack.getItem`, `ItemStack.count` and
/// `EnchantmentHelper.getItemEnchantmentLevel` over
/// `DataComponents.ENCHANTMENTS`.
pub(crate) fn loot_tool_for_persistent_stack(
    stack: &RuntimeItemStack,
    registry: &[crate::enchantment_facts::EnchantmentSlots],
) -> LootTool {
    if stack.is_empty() {
        return LootTool::empty();
    }
    LootTool {
        item_id: stack.item_id(),
        count: stack.count(),
        enchantments: crate::entity::item_components::enchantments(stack, registry)
            .into_iter()
            .collect(),
        stack: stack.clone(),
    }
}

fn drop_custom_death_loot(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    player_killed: bool,
) -> Option<()> {
    let position = victim.position();
    for entry in bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS {
        let slot = entry.slot;
        let EntityData::GeneralLiving(data) = victim.data() else {
            return Some(());
        };
        let Some(mob) = data.mob.as_ref() else {
            return Some(());
        };
        let Some(stack) = victim
            .living()
            .and_then(|living| living.equipment().get(slot))
            .filter(|stack| !stack.is_empty())
            .cloned()
        else {
            continue;
        };
        let drop_chance = mob.drop_chance(slot);
        if drop_chance == 0.0 {
            continue;
        }
        let preserve = drop_chance > 1.0;
        let drop_chance =
            match modified_equipment_drop_chance(store, level, victim, &source, drop_chance) {
                Ok(chance) => chance,
                Err(detail) => {
                    tracing::warn!("equipment drop chance left unmodified: {detail}");
                    drop_chance
                }
            };
        if crate::entity::armor_stand::stack_enchantment_ids(&stack, store)
            .iter()
            .any(|id| store.prevents_equipment_drop(id))
        {
            continue;
        }
        if !player_killed && !preserve {
            continue;
        }
        if store.random().next_float().partial_cmp(&drop_chance) != Some(std::cmp::Ordering::Less) {
            continue;
        }
        let mut original_item = stack.clone();
        if !preserve {
            if let Some(damage) = randomize_damage(&original_item, store)? {
                original_item = original_item
                    .with_component_nbt(
                        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
                        Some(&NbtSummaryValue::Int(damage)),
                        &bbb_registry_codec::UnavailableHolderResolver,
                    )
                    .ok()?;
            }
        }
        level.spawn_runtime_item(position, original_item, true)?;
        crate::entity::item_slots::set_living_item_in_tick(victim, store, level, slot, None)
            .ok()?;
    }
    // `WitherBoss.dropCustomDeathLoot` (`WitherBoss.java:488-494`).
    if victim.entity_type_id() == VANILLA_ENTITY_TYPE_WITHER_ID {
        let star = crate::entity::item::plain_stack("minecraft:nether_star", 1)?;
        crate::entity::item::spawn_extended_lifetime_item(store, position, star)?;
    }
    // Piglin.java:128-131.
    if victim.entity_type_id() == VANILLA_ENTITY_TYPE_PIGLIN_ID {
        if let EntityData::GeneralLiving(data) = &mut victim.data {
            if let Some(inventory) = data.internal_inventory_mut() {
                for (_, stack) in inventory.remove_all_items() {
                    level.spawn_runtime_item(position, stack, true)?;
                }
            }
        }
    }
    if let EntityData::GeneralLiving(data) = victim.data() {
        if let Some(state) = data
            .enderman_retained()
            .and_then(|state| state.carried_block_state_id())
        {
            let drops = level.enderman_block_loot_drops(
                EntityBodyReadView::new(store, Some(victim), None),
                victim,
                state,
            );
            for stack in drops.into_iter().flatten() {
                level.spawn_runtime_item(position, RuntimeItemStack::from_summary(stack), true)?;
            }
        }
    }
    Some(())
}

fn modified_equipment_drop_chance(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
    source: &StoredEntityDamageSource<'_>,
    chance: f32,
) -> Result<f32, String> {
    let slots = attacker_slots(source);
    // A mob mid-swing or a ticking cloud is lifted out of the store.
    let [causing_body, direct_body] = slots.map(|slot| match slot {
        AttackerSlot::Stored(id) => store
            .get(id)
            .or_else(|| (id == victim.id()).then_some(victim))
            .or_else(|| level.detached_entity(id)),
        _ => None,
    });
    let views = [source.attacker_views.causing, source.attacker_views.direct];
    let attacker_living = match slots[0] {
        AttackerSlot::None => false,
        AttackerSlot::Player(_) => true,
        AttackerSlot::Stored(_) => causing_body.map_or(
            views[0].and_then(|facts| facts.living) == Some(true),
            |entity| entity.living().is_some(),
        ),
    };
    // Mob.java:862-872 passes the living causing source as the helper subject.
    if !attacker_living {
        return Ok(chance);
    }
    let [attacker, direct_attacking_entity] = [0, 1].map(|index| {
        let body = [causing_body, direct_body][index];
        match slots[index] {
            AttackerSlot::Player(uuid) => MobLootAttacker::SessionPlayer { uuid },
            slot => attacker_slot(
                slot,
                body.map(|entity| stored_loot_entity(store, level, entity)),
                views[index],
            ),
        }
    });
    let bodies = crate::entity::store::EntityBodyReadView::new(store, Some(victim), None);
    level.equipment_drop_chance(EquipmentDropChanceRequest {
        bodies,
        subject: attacker,
        original_chance: chance,
        attacker,
        attacker_living,
        direct_attacking_entity,
        damage_source: MobLootDamageSource::of(source),
    })
}

pub(crate) fn with_enchanted_damage_request<R>(
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: &Entity,
    source: &StoredEntityDamageSource<'_>,
    items: crate::entity::enchanted_damage::EnchantedDamageItems<'_>,
    read: impl FnOnce(
        &mut LevelCtx<'_, '_>,
        crate::entity::enchanted_damage::EnchantedDamageRequest<'_>,
    ) -> R,
) -> R {
    let slots = attacker_slots(source);
    let victim_entity = stored_loot_entity(store, level, victim);
    let [causing, direct] = [0, 1].map(|index| {
        let stored = match slots[index] {
            AttackerSlot::Stored(id) => store
                .get(id)
                .or_else(|| (id == victim.id()).then_some(victim))
                .map(|entity| stored_loot_entity(store, level, entity)),
            _ => None,
        };
        let lent = if index == 0 {
            source.attacker_views.causing
        } else {
            source.attacker_views.direct
        };
        attacker_slot(slots[index], stored, lent)
    });
    read(
        level,
        crate::entity::enchanted_damage::EnchantedDamageRequest {
            bodies: EntityBodyReadView::new(store, Some(victim), None),
            items,
            victim: victim_entity,
            causing,
            direct,
            damage_source: MobLootDamageSource::of(source),
        },
    )
}

fn randomize_damage(stack: &RuntimeItemStack, store: &EntityStore) -> Option<Option<i32>> {
    let Some((_, max_damage)) = crate::entity::item_components::damage_and_max(stack).ok()? else {
        return Some(None);
    };
    let random = store.random();
    let inner = random.next_int((max_damage - 3).max(1));
    Some(Some(max_damage - random.next_int(1 + inner)))
}

fn create_wither_rose(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, victim: &mut Entity) {
    if !kill_credit_is_wither(victim, store, level) {
        return;
    }
    let (pos, position) = (victim.block_position(), victim.position());
    place_wither_rose(store, level, Some(victim), pos, position);
}

/// `LivingEntity.createWitherRose`'s body (LivingEntity.java:1459-1475) once
/// the killer is known to be a wither.
pub(crate) fn place_wither_rose(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    victim: Option<&Entity>,
    pos: bbb_protocol::packets::BlockPos,
    position: bbb_protocol::packets::Vec3d,
) {
    if level.mob_griefing()
        && block_state_is_air(level.block_state(pos))
        && rose_can_survive(level, pos)
    {
        if let Some(state) =
            vanilla_block_state_id_by_name_and_properties(WITHER_ROSE_BLOCK, &BTreeMap::new())
        {
            level.set_entity_block(
                crate::entity::level::EntityBlockContext::new(Some(&mut *store), victim),
                pos,
                state,
                SET_BLOCK_UPDATE_ALL,
            );
            return;
        }
    }
    let Some(item) = bbb_protocol::item_types::vanilla_item_type_for_resource_id(WITHER_ROSE_ITEM)
    else {
        return;
    };
    let stack = ItemStackSummary {
        item_id: Some(item.id),
        count: 1,
        component_patch: Default::default(),
    };
    level.spawn_item_without_pickup_delay(position, &stack);
}

fn rose_can_survive(level: &mut LevelCtx<'_, '_>, pos: bbb_protocol::packets::BlockPos) -> bool {
    let mut block_state_at = |at: bbb_protocol::packets::BlockPos| level.block_state(at);
    may_place_on(
        VegetationSupport::WitherRose,
        pos,
        level.tags(),
        &mut block_state_at,
    ) == Some(true)
}

fn kill_credit_is_wither(victim: &Entity, store: &EntityStore, level: &LevelCtx<'_, '_>) -> bool {
    let Some(living) = victim.living() else {
        return false;
    };
    if living.last_hurt_by_player.is_some() {
        return false;
    }
    living
        .last_hurt_by_mob()
        .and_then(|reference| reference.resolve(store, level))
        .is_some_and(|killer| killer.entity_type_id == WITHER_ENTITY_TYPE_ID)
}

#[cfg(test)]
mod fire_flags_tests;
