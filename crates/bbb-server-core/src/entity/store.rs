//! Entity identity and per-section storage.
//!
//! This is the port of `PersistentEntitySectionManager` over
//! `EntitySectionStorage` (`net/minecraft/world/level/entity/`): the thing that
//! knows which entities exist, what their ids are, and which 16x16x16 section
//! each one currently belongs to.
//!
//! # What is simplified, and what is not
//!

mod item_references;
mod journals;
pub use item_references::EquipmentItemReference;
mod visibility;
pub use visibility::EntityVisibility;
use visibility::EntityVisibilityState;
mod sections;
use sections::SectionEntities;
pub use sections::{DetachedEntityBodies, EntityBodyReadView, SmallIds};
mod ai_runtime;
mod mob_pickup;
mod passengers;
pub use mob_pickup::PendingThrownItemPickup;
mod commands;
pub use commands::EntityCommand;

use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

use crate::random::GameRandom;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::vanilla_entity_type;
use bbb_protocol::packets::{BlockPos, Vec3d};
use uuid::Uuid;

use crate::connection::ClientboundPlayPacket;
use crate::entity::ai::mob::MobAiRuntime;
use crate::entity::{Entity, EntityData, RemovalReason};
use crate::level_ticks::ChunkKey;
use bbb_server_types::geometry::Aabb;

mod cross_level;
mod fresh;
mod hanging_removal;

pub use cross_level::{
    transfer_stored_entity_subtree_cross_level,
    transfer_stored_entity_subtree_cross_level_with_context,
    transfer_stored_entity_tree_cross_level, transfer_stored_entity_tree_cross_level_with_context,
    CrossLevelEntityTreeError, CrossLevelEntityTreeOutcome, StoredEntityTransition,
};

/// A packed 64-bit section key matching `SectionPos.asLong` (`SectionPos.java:216-221`):
/// 22 signed bits X (`42..64`), 22 bits Z (`20..42`), 20 bits Y (`0..20`).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SectionKey(i64);

impl SectionKey {
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(
            (((x as i64) & 0x3f_ffff) << 42)
                | (((z as i64) & 0x3f_ffff) << 20)
                | ((y as i64) & 0xf_ffff),
        )
    }

    #[inline]
    pub const fn as_long(self) -> i64 {
        self.0
    }

    #[inline]
    pub const fn x(self) -> i32 {
        (self.0 >> 42) as i32
    }

    #[inline]
    pub const fn y(self) -> i32 {
        ((self.0 << 44) >> 44) as i32
    }

    #[inline]
    pub const fn z(self) -> i32 {
        ((self.0 << 22) >> 42) as i32
    }

    #[inline]
    pub const fn column(self) -> (i32, i32) {
        (self.x(), self.z())
    }
}

impl std::fmt::Debug for SectionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SectionKey")
            .field(&self.x())
            .field(&self.y())
            .field(&self.z())
            .finish()
    }
}

impl From<(i32, i32, i32)> for SectionKey {
    #[inline]
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        Self::new(x, y, z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramedMapRemoval {
    pub map_id: i32,
    pub pos: BlockPos,
    pub entity_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalPlayerVehicleUnload {
    DetachedOnly {
        direct_vehicle_id: i32,
        root_vehicle_id: i32,
    },
    UnloadedRootTree {
        direct_vehicle_id: i32,
        root_vehicle_id: i32,
        entity_ids: Vec<i32>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalPlayerDismount {
    pub vehicle_id: i32,
    pub vehicle_position: Vec3d,
}

/// The ordered lifecycle callbacks a consumer must observe around one store.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityLifecycleEvent {
    Added {
        entity: Entity,
        tracked: bool,
    },
    TrackingStarted {
        entity: Entity,
    },
    TrackingStopped {
        entity: Entity,
    },
    Moved {
        entity_id: i32,
        from: SectionKey,
        to: SectionKey,
    },
    Removed {
        entity: Entity,
        tracked: bool,
    },
    Broadcast {
        entity_id: i32,
        packet: ClientboundPlayPacket,
    },
    BlockDestruction {
        entity_id: i32,
        pos: bbb_protocol::packets::BlockPos,
        stage: i32,
    },
    PassengersChanged {
        vehicle_id: i32,
        passenger_ids: Vec<i32>,
    },
    WaypointUpdated {
        entity_id: i32,
    },
    /// The external Player owner applies this after the preceding relation edge.
    ExternalPlayerBoardingCooldownReset {
        player_entity_id: i32,
        dismount: Option<ExternalPlayerDismount>,
    },
    Sound {
        sound: bbb_protocol::packets::SoundEvent,
    },
    SoundExcept {
        sound: bbb_protocol::packets::SoundEvent,
        except_entity_id: i32,
    },
    SoundFromEntity {
        sound: bbb_protocol::packets::SoundEntityEvent,
        position_at_emit: Vec3d,
    },
    Particles {
        particles: bbb_protocol::packets::LevelParticles,
    },
    MobEffectChanged {
        recipient_entity_ids: Vec<i32>,
        packet: ClientboundPlayPacket,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingEntitySound {
    pub position: Vec3d,
    pub resource_id: String,
    pub source: bbb_protocol::packets::SoundSource,
    pub volume: f32,
    pub pitch: f32,
}

/// A registered sound by reference, any other id (a datapack's custom
/// `equip_sound`, say) as a direct event the client resolves.
pub(crate) fn sound_holder_for_id(
    resource_id: &str,
) -> Option<bbb_protocol::packets::SoundEventHolder> {
    Some(
        match bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id) {
            Some(registry_id) => bbb_protocol::packets::SoundEventHolder::Reference { registry_id },
            None if resource_id.is_empty() => return None,
            None => bbb_protocol::packets::SoundEventHolder::Direct {
                location: resource_id.to_owned(),
                fixed_range: None,
            },
        },
    )
}

/// `SectionPos.blockToSectionCoord(int)`
/// (`net/minecraft/core/SectionPos.java:80-82`).
const fn block_to_section_coord(block: i32) -> i32 {
    block >> 4
}

fn pos_to_section_coord(pos: f64) -> i32 {
    block_to_section_coord(pos.floor() as i32)
}

/// Signed `SectionPos.asLong` ordering (`SectionPos.java:216-221`), matching the
/// sorted `LongAVLTreeSet` behind `forEachAccessibleNonEmptySection`.
#[inline]
pub const fn section_walk_order_key(section: SectionKey) -> i64 {
    section.as_long()
}

/// The section `SectionPos.asLong(entity.blockPosition())` names
/// (`PersistentEntitySectionManager.java:79`).
#[inline]
pub const fn section_of_block(pos: BlockPos) -> SectionKey {
    SectionKey::new(
        block_to_section_coord(pos.x),
        block_to_section_coord(pos.y),
        block_to_section_coord(pos.z),
    )
}

pub(crate) enum EntityRestoreOutcome {
    Published(i32),
    RejectedRemoved,
    RejectedDuplicateUuid,
}

///
/// It is a handle rather than a global so a test can hold its own sequence, and
/// it is shared rather than copied because vanilla's is static: an id is unique
/// across every level of a server, not only within one. `ServerWorld` hands the
/// same counter to each level's store and to its own player avatars.
#[derive(Debug, Clone, Default)]
pub struct EntityIdCounter(Arc<AtomicI32>);

impl EntityIdCounter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_id(&self) -> i32 {
        // `AtomicInteger.incrementAndGet` wraps on overflow the same way.
        self.0.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
    }
}

impl PartialEq for EntityIdCounter {
    /// Two handles are equal when they are the same counter, which is what a
    /// store comparison wants; the live value is not part of a store's
    /// identity.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The registry generation item stacks in this store name their holders
/// against when they are read from or written to a save.
#[derive(Clone)]
pub struct ItemPublication(
    Arc<dyn bbb_registry_codec::PersistentItemStackHolderResolver + Send + Sync>,
);

impl ItemPublication {
    pub fn new(
        resolver: Arc<dyn bbb_registry_codec::PersistentItemStackHolderResolver + Send + Sync>,
    ) -> Self {
        Self(resolver)
    }

    pub fn resolver(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        &*self.0
    }
}

impl Default for ItemPublication {
    fn default() -> Self {
        Self(Arc::new(bbb_registry_codec::UnavailableHolderResolver))
    }
}

impl PartialEq for ItemPublication {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl std::fmt::Debug for ItemPublication {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ItemPublication")
    }
}

/// One stored entity: its body and the runtime state the store keeps beside it.
///
/// The slot outlives a body lent out for its own tick ([`EntityStore::detach`]),
/// so the AI runtime and riding relation stay reachable by id meanwhile.
#[derive(Default)]
struct EntitySlot {
    body: Option<Box<Entity>>,
    ai: Option<Box<MobAiRuntime>>,
    vehicle: Option<i32>,
    passengers: Vec<i32>,
    /// The removal edge was already published while the body was detached.
    removal_published: bool,
}

impl std::fmt::Debug for EntitySlot {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EntitySlot")
            .field("body", &self.body)
            .field("vehicle", &self.vehicle)
            .field("passengers", &self.passengers)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct StoredPlayers(FxHashMap<i32, Box<crate::world::PlayerState>>);

impl std::ops::Deref for StoredPlayers {
    type Target = FxHashMap<i32, Box<crate::world::PlayerState>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for StoredPlayers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::fmt::Debug for StoredPlayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredPlayers")
            .field("count", &self.0.len())
            .finish()
    }
}

/// Every entity in one level, indexed by id and by section.
#[derive(Debug)]
pub struct EntityStore {
    item_publication: ItemPublication,
    visibility: EntityVisibilityState,
    entities: FxHashMap<i32, EntitySlot>,
    /// Online players indexed by entity id (separate from `entities` so player
    /// iteration is O(players) without scanning stored bodies).
    players: StoredPlayers,
    /// Live stored bodies currently attached (`slot.body.is_some()`), for O(1) len/is_empty.
    live_body_count: usize,
    by_section: FxHashMap<SectionKey, SectionEntities>,
    sections_by_column: FxHashMap<ChunkKey, Vec<SectionKey>>,
    /// Monotonic counter feeding [`Self::chunk_dirty_epochs`] and [`Self::global_dirty_epoch`].
    next_chunk_dirty_epoch: u64,
    /// Store-wide floor bumped when publication/plugin metadata changes.
    global_dirty_epoch: u64,
    /// O(1) per-chunk entity dirty epoch bumped on add/remove/mutate/section-move.
    chunk_dirty_epochs: FxHashMap<ChunkKey, u64>,
    ids_by_uuid: FxHashMap<Uuid, i32>,
    dragon_ids: BTreeSet<i32>,
    /// Live thrown ender pearls, for O(#pearls) chunk-ticket refreshes.
    ender_pearl_ids: FxHashSet<i32>,
    /// Live end crystals, for O(#crystals) dragon-fight fire maintenance.
    end_crystal_ids: FxHashSet<i32>,
    /// Entity ids whose AI runtime is active, for avoiding full-store scans on block edits.
    ai_entity_ids: FxHashSet<i32>,
    /// Active fishing bobber entity id indexed by owner UUID.
    bobbers_by_owner: FxHashMap<Uuid, i32>,
    /// The slots holding an online player, by profile UUID.
    players_by_uuid: FxHashMap<Uuid, i32>,
    /// Session players riding a stored vehicle, keyed by player entity id.
    lifecycle_events: Vec<EntityLifecycleEvent>,
    /// How far into `lifecycle_events` the level's game-event listener
    /// registry has applied the add/move/remove edges.
    listener_sync_cursor: usize,
    entity_path_debug_capture: bool,
    ids: EntityIdCounter,
    /// The level's gameplay stream.
    random: GameRandom,
    /// The last `Level.getGameTime()` [`crate::entity::tick_entities`] saw.
    game_time: i64,
    /// The difficulty captured before the last world entity loop.
    difficulty: Option<Difficulty>,
    /// The level facts target goals read, captured before the last world
    /// entity loop.
    target_level_facts: crate::entity::TargetLevelFacts,
    /// The level's sessions captured before the last world entity loop, which
    /// `LivingEntity.pushEntities` meets beside the stored bodies.
    prevent_equipment_drop_enchantments: Arc<BTreeSet<String>>,
    enchantment_slots: Arc<Vec<crate::enchantment_facts::EnchantmentSlots>>,
    /// The generation's `minecraft:attributes` enchantment effects, which equipped
    /// stacks add to their wearer.
    enchantment_attributes: Arc<crate::enchantment_facts::EnchantmentAttributeTable>,
    pub(crate) enchantment_location_effects:
        Arc<crate::enchantment_location_effects::EnchantmentLocationTable>,
    pub(crate) enchantment_tick_effects:
        Arc<crate::enchantment_location_effects::EnchantmentTickTable>,
    weapon_effects: Arc<crate::enchantment_weapon_effects::WeaponEffectTable>,
    commands: Vec<EntityCommand>,
    prevent_armor_change: Arc<BTreeSet<String>>,
    /// Charged creepers whose blast this game tick may still drop a head
    /// (`Creeper.killedEntity`); the body is gone before the blast resolves.
    charged_creeper_blasts: Vec<(i32, i64)>,
    plugins: Arc<crate::plugin::ServerPluginRegistry>,
}

/// One stored entity's death as `LivingEntity.die` states it to the players
/// it concerns.
///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPlayerStat {
    pub player_uuid: Uuid,
    pub stat: &'static str,
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingPlayerHurtEntity {
    pub player_uuid: Uuid,
    pub victim_entity_id: i32,
    pub damage_type_id: i32,
    pub direct_entity_id: Option<i32>,
    pub dealt: f32,
    pub taken: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPlayerKill {
    pub kill_credit_player_uuid: Option<Uuid>,
    pub statistic_player_uuid: Option<Uuid>,
    pub victim_type_id: i32,
    pub victim_entity_id: i32,
    /// Equipment before death drops clear the victim's slots.
    pub victim_equipment: Option<
        BTreeMap<
            bbb_protocol::equipment_slots::EquipmentSlot,
            bbb_server_types::item_stack::RuntimeItemStack,
        >,
    >,
    pub killing_blow: PendingKillingBlow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTamedAnimal {
    pub player_uuid: Uuid,
    /// The tamed animal's entity id; it is still filed in the store when the
    /// record is drained, because the taming interaction and the drain are the
    /// same frame.
    pub entity_id: i32,
    /// `EntityType.getKey(this.getType())`, for a consumer that only needs the
    /// type.
    pub entity_type_id: i32,
}

/// `Animal.finalizeSpawnChildFromBreeding`'s love-cause half
/// (`Animal.java:220-223`): `Stats.ANIMALS_BRED` and `BRED_ANIMALS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingBredAnimals {
    pub player_uuid: Uuid,
    pub parent_id: i32,
    pub partner_id: i32,
    pub child_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingKillingBlow {
    pub damage_type: String,
    pub direct_entity_id: Option<i32>,
    pub causing_entity_id: Option<i32>,
    pub is_direct: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct EntityLifecycleCheckpoint(usize);

impl EntityStore {
    /// A store drawing from a fresh stream seeded with `seed`.
    pub fn new(seed: i64, ids: EntityIdCounter) -> Self {
        Self::with_random(GameRandom::with_seed(seed), ids)
    }

    pub(crate) fn with_random(random: GameRandom, ids: EntityIdCounter) -> Self {
        Self {
            visibility: EntityVisibilityState::default(),
            entities: FxHashMap::default(),
            players: StoredPlayers::default(),
            live_body_count: 0,
            by_section: FxHashMap::default(),
            sections_by_column: FxHashMap::default(),
            next_chunk_dirty_epoch: 0,
            global_dirty_epoch: 0,
            chunk_dirty_epochs: FxHashMap::default(),
            ids_by_uuid: FxHashMap::default(),
            dragon_ids: BTreeSet::new(),
            ender_pearl_ids: FxHashSet::default(),
            end_crystal_ids: FxHashSet::default(),
            ai_entity_ids: FxHashSet::default(),
            bobbers_by_owner: FxHashMap::default(),
            players_by_uuid: FxHashMap::default(),
            lifecycle_events: Vec::new(),
            listener_sync_cursor: 0,
            entity_path_debug_capture: false,
            ids,
            random,
            game_time: 0,
            difficulty: None,
            target_level_facts: crate::entity::TargetLevelFacts::default(),
            item_publication: ItemPublication::default(),
            prevent_equipment_drop_enchantments:
                crate::enchantment_facts::load_enchantments_with_effect(
                    None,
                    crate::enchantment_facts::PREVENT_EQUIPMENT_DROP_EFFECT,
                ),
            enchantment_slots: crate::enchantment_facts::load_enchantment_slots(None),
            enchantment_attributes: crate::enchantment_facts::load_enchantment_attribute_effects(
                None,
            ),
            enchantment_location_effects:
                crate::enchantment_location_effects::load_enchantment_location_effects(None),
            enchantment_tick_effects:
                crate::enchantment_location_effects::load_enchantment_tick_effects(None),
            weapon_effects: crate::enchantment_weapon_effects::load_weapon_effects(None),
            commands: Vec::new(),
            prevent_armor_change: crate::enchantment_facts::load_enchantments_with_effect(
                None,
                crate::enchantment_facts::PREVENT_ARMOR_CHANGE_EFFECT,
            ),
            charged_creeper_blasts: Vec::new(),
            plugins: Arc::clone(crate::plugin::ServerPluginRegistry::vanilla()),
        }
    }

    pub fn plugins(&self) -> &Arc<crate::plugin::ServerPluginRegistry> {
        &self.plugins
    }

    pub fn set_plugins(&mut self, plugins: Arc<crate::plugin::ServerPluginRegistry>) {
        self.plugins = plugins;
        self.mark_all_chunks_dirty();
    }

    /// Current monotonic dirty epoch for entities serialized in `chunk`.
    pub fn chunk_dirty_epoch(&self, chunk: ChunkKey) -> u64 {
        self.chunk_dirty_epochs
            .get(&chunk)
            .copied()
            .unwrap_or(0)
            .max(self.global_dirty_epoch)
    }

    fn advance_dirty_epoch(&mut self) -> u64 {
        self.next_chunk_dirty_epoch = self.next_chunk_dirty_epoch.wrapping_add(1);
        if self.next_chunk_dirty_epoch == 0 {
            self.next_chunk_dirty_epoch = 1;
        }
        self.next_chunk_dirty_epoch
    }

    pub(super) fn mark_chunk_dirty(&mut self, chunk: ChunkKey) {
        let epoch = self.advance_dirty_epoch();
        self.chunk_dirty_epochs.insert(chunk, epoch);
    }

    pub(super) fn mark_all_chunks_dirty(&mut self) {
        self.global_dirty_epoch = self.advance_dirty_epoch();
    }

    pub(super) fn mark_entity_chunk_dirty(&mut self, id: i32) {
        let chunk = self
            .entities
            .get(&id)
            .and_then(|slot| slot.body.as_deref())
            .map(|entity| entity.filed_section().column());
        if let Some(chunk) = chunk {
            self.mark_chunk_dirty(chunk);
        }
        // Passengers serialize under their root vehicle's chunk, so any mutation
        // in the riding chain also dirties that root chunk.
        let root = self.root_vehicle(id);
        if root != id {
            if let Some(root_chunk) = self
                .entities
                .get(&root)
                .and_then(|slot| slot.body.as_deref())
                .map(|entity| entity.filed_section().column())
            {
                if Some(root_chunk) != chunk {
                    self.mark_chunk_dirty(root_chunk);
                }
            }
        }
    }

    /// Lightweight scratch store for command NBT decode that reuses the live
    /// store's plugin registry and pre-parsed enchantment tables instead of
    /// reparsing builtin JSON on every decode attempt.
    pub(crate) fn for_command_decode(&self, game_time: i64) -> Self {
        Self {
            visibility: EntityVisibilityState::default(),
            entities: FxHashMap::default(),
            players: StoredPlayers::default(),
            live_body_count: 0,
            by_section: FxHashMap::default(),
            sections_by_column: FxHashMap::default(),
            next_chunk_dirty_epoch: 0,
            global_dirty_epoch: 0,
            chunk_dirty_epochs: FxHashMap::default(),
            ids_by_uuid: FxHashMap::default(),
            dragon_ids: BTreeSet::new(),
            ender_pearl_ids: FxHashSet::default(),
            end_crystal_ids: FxHashSet::default(),
            ai_entity_ids: FxHashSet::default(),
            bobbers_by_owner: FxHashMap::default(),
            players_by_uuid: FxHashMap::default(),
            lifecycle_events: Vec::new(),
            listener_sync_cursor: 0,
            entity_path_debug_capture: false,
            ids: EntityIdCounter::new(),
            random: GameRandom::with_seed(0),
            game_time,
            difficulty: self.difficulty,
            target_level_facts: crate::entity::TargetLevelFacts::default(),
            item_publication: self.item_publication.clone(),
            prevent_equipment_drop_enchantments: Arc::clone(
                &self.prevent_equipment_drop_enchantments,
            ),
            enchantment_slots: Arc::clone(&self.enchantment_slots),
            enchantment_attributes: Arc::clone(&self.enchantment_attributes),
            enchantment_location_effects: Arc::clone(&self.enchantment_location_effects),
            enchantment_tick_effects: Arc::clone(&self.enchantment_tick_effects),
            weapon_effects: Arc::clone(&self.weapon_effects),
            commands: Vec::new(),
            prevent_armor_change: Arc::clone(&self.prevent_armor_change),
            charged_creeper_blasts: Vec::new(),
            plugins: Arc::clone(&self.plugins),
        }
    }

    pub fn set_item_publication(&mut self, publication: ItemPublication) {
        self.item_publication = publication;
        self.mark_all_chunks_dirty();
    }

    pub fn item_publication(&self) -> ItemPublication {
        self.item_publication.clone()
    }

    pub fn item_resolver(&self) -> &dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.item_publication.resolver()
    }

    pub fn set_prevent_equipment_drop_enchantments(&mut self, enchantments: Arc<BTreeSet<String>>) {
        self.prevent_equipment_drop_enchantments = enchantments;
    }

    pub fn prevents_equipment_drop(&self, enchantment_id: &str) -> bool {
        self.prevent_equipment_drop_enchantments
            .contains(enchantment_id)
    }

    pub fn set_enchantment_slots(
        &mut self,
        slots: Arc<Vec<crate::enchantment_facts::EnchantmentSlots>>,
    ) {
        self.enchantment_slots = slots;
    }

    pub fn set_enchantment_attribute_effects(
        &mut self,
        effects: Arc<crate::enchantment_facts::EnchantmentAttributeTable>,
    ) {
        self.enchantment_attributes = effects;
    }

    #[must_use]
    pub fn enchantment_attribute_effects(
        &self,
    ) -> &crate::enchantment_facts::EnchantmentAttributeTable {
        &self.enchantment_attributes
    }

    #[must_use]
    pub fn enchantment_attribute_effects_shared(
        &self,
    ) -> Arc<crate::enchantment_facts::EnchantmentAttributeTable> {
        Arc::clone(&self.enchantment_attributes)
    }

    pub fn set_weapon_effects(
        &mut self,
        effects: Arc<crate::enchantment_weapon_effects::WeaponEffectTable>,
    ) {
        self.weapon_effects = effects;
    }

    /// The generation's weapon enchantment components in holder-id order.
    #[must_use]
    pub fn weapon_effects(&self) -> &crate::enchantment_weapon_effects::WeaponEffectTable {
        &self.weapon_effects
    }

    pub(crate) fn weapon_effects_shared(
        &self,
    ) -> Arc<crate::enchantment_weapon_effects::WeaponEffectTable> {
        Arc::clone(&self.weapon_effects)
    }

    /// The generation's `minecraft:enchantment` registry in holder-id order.
    pub fn enchantment_registry(&self) -> &[crate::enchantment_facts::EnchantmentSlots] {
        &self.enchantment_slots
    }

    pub(crate) fn enchantment_slots(
        &self,
        enchantment_id: &str,
    ) -> Option<&[bbb_protocol::equipment_slots::EquipmentSlot]> {
        self.enchantment_slots
            .iter()
            .find(|entry| entry.resource_id == enchantment_id)
            .map(|entry| entry.slots.as_slice())
    }

    /// `Level.getGameTime()` as of the last entity loop.
    pub const fn game_time(&self) -> i64 {
        self.game_time
    }

    pub fn set_game_time(&mut self, game_time: i64) {
        self.game_time = game_time;
    }

    pub(crate) fn add_charged_creeper_blast(&mut self, creeper_id: i32) {
        let now = self.game_time;
        self.charged_creeper_blasts.retain(|(_, at)| *at == now);
        self.charged_creeper_blasts.push((creeper_id, now));
    }

    pub(crate) fn is_charged_creeper_blast(&self, creeper_id: i32) -> bool {
        self.charged_creeper_blasts
            .contains(&(creeper_id, self.game_time))
    }

    pub(crate) fn spend_charged_creeper_blast(&mut self, creeper_id: i32) {
        self.charged_creeper_blasts
            .retain(|(id, _)| *id != creeper_id);
    }

    pub const fn difficulty(&self) -> Option<Difficulty> {
        self.difficulty
    }

    pub fn set_difficulty(&mut self, difficulty: Difficulty) {
        self.difficulty = Some(difficulty);
    }

    pub const fn target_level_facts(&self) -> crate::entity::TargetLevelFacts {
        self.target_level_facts
    }

    pub fn set_target_level_facts(&mut self, facts: crate::entity::TargetLevelFacts) {
        self.target_level_facts = facts;
    }

    ///
    /// A sound outside the built-in registry is dropped: `SoundEvent.CODEC`
    /// would not have resolved it either.
    pub fn play_entity_sound(&mut self, sound: PendingEntitySound) {
        let seed = self.random.next_long();
        self.play_entity_sound_with_seed(sound, seed);
    }

    pub fn play_entity_sound_with_seed(&mut self, sound: PendingEntitySound, seed: i64) {
        let Some(holder) = sound_holder_for_id(&sound.resource_id) else {
            return;
        };
        self.emit_entity_sound(bbb_protocol::packets::SoundEvent {
            sound: holder,
            source: sound.source,
            position: sound.position,
            volume: sound.volume,
            pitch: sound.pitch,
            seed,
        });
    }

    pub fn play_entity_sound_except(&mut self, except_entity_id: i32, sound: PendingEntitySound) {
        let seed = self.random.next_long();
        let Some(holder) = sound_holder_for_id(&sound.resource_id) else {
            return;
        };
        self.lifecycle_events
            .push(EntityLifecycleEvent::SoundExcept {
                sound: bbb_protocol::packets::SoundEvent {
                    sound: holder,
                    source: sound.source,
                    position: sound.position,
                    volume: sound.volume,
                    pitch: sound.pitch,
                    seed,
                },
                except_entity_id,
            });
    }

    pub(crate) fn emit_entity_sound(&mut self, sound: bbb_protocol::packets::SoundEvent) {
        self.lifecycle_events
            .push(EntityLifecycleEvent::Sound { sound });
    }

    pub fn play_sound_from_entity(
        &mut self,
        entity_id: i32,
        resource_id: &str,
        source: bbb_protocol::packets::SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        let Some(position_at_emit) = self.get(entity_id).map(Entity::position) else {
            return false;
        };
        self.play_sound_from_entity_at(
            entity_id,
            position_at_emit,
            resource_id,
            source,
            volume,
            pitch,
        )
    }

    pub fn play_sound_from_entity_at(
        &mut self,
        entity_id: i32,
        position_at_emit: Vec3d,
        resource_id: &str,
        source: bbb_protocol::packets::SoundSource,
        volume: f32,
        pitch: f32,
    ) -> bool {
        let Some(registry_id) =
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(resource_id)
        else {
            return false;
        };
        self.play_sound_holder_from_entity_at(
            entity_id,
            position_at_emit,
            bbb_protocol::packets::SoundEventHolder::Reference { registry_id },
            source,
            volume,
            pitch,
        );
        true
    }

    pub(crate) fn play_sound_holder_from_entity_at(
        &mut self,
        entity_id: i32,
        position_at_emit: Vec3d,
        sound: bbb_protocol::packets::SoundEventHolder,
        source: bbb_protocol::packets::SoundSource,
        volume: f32,
        pitch: f32,
    ) {
        let seed = self.random.next_long();
        self.lifecycle_events
            .push(EntityLifecycleEvent::SoundFromEntity {
                sound: bbb_protocol::packets::SoundEntityEvent {
                    sound,
                    source,
                    entity_id,
                    volume,
                    pitch,
                    seed,
                },
                position_at_emit,
            });
    }

    pub fn broadcast_to_tracking(&mut self, entity_id: i32, packet: ClientboundPlayPacket) {
        self.lifecycle_events
            .push(EntityLifecycleEvent::Broadcast { entity_id, packet });
    }

    pub fn send_particles(&mut self, particles: bbb_protocol::packets::LevelParticles) {
        self.lifecycle_events
            .push(EntityLifecycleEvent::Particles { particles });
    }

    fn allocate_id(&mut self) -> i32 {
        self.ids.next_id()
    }

    pub fn spawn(&mut self, entity_type_id: i32, position: Vec3d, data: EntityData) -> i32 {
        let uuid = self.next_uuid();
        self.restore(uuid, entity_type_id, position, data)
            .expect("spawn requires matching catalog data and a fresh UUID")
    }

    /// A fresh random UUID. It comes from OS entropy rather than the level
    /// stream: the stream restarts at the same seed on every boot, and a
    /// repeated UUID would refuse an entity saved in an unloaded chunk.
    pub fn next_uuid(&mut self) -> Uuid {
        Uuid::new_v4()
    }

    /// The level's gameplay stream.
    pub fn random(&self) -> &GameRandom {
        &self.random
    }

    /// `PersistentEntitySectionManager.addEntity(entity, loaded = true)`
    /// (`:74-96`), reached from `addLegacyChunkEntities` and the pending-load
    /// queue: the saved UUID is kept, and a duplicate is refused.
    ///
    ///
    /// `None` for a UUID this store already knows, which is vanilla's
    /// `addEntityUuid` refusal (`:61-68`), and for an entity type outside the
    /// built-in 26.1 registry, or whose data names a different catalog type.
    /// Vanilla cannot express either because its `EntityType` is one registry
    /// object shared by the entity and its serializer.
    pub fn restore(
        &mut self,
        uuid: Uuid,
        entity_type_id: i32,
        position: Vec3d,
        data: EntityData,
    ) -> Option<i32> {
        self.restore_initialized(uuid, entity_type_id, position, data, |_| {})
    }

    /// Restores an entity and finishes its construction before publishing it.
    ///
    ///
    /// Type and UUID refusal happen before allocating or constructing an
    /// entity. The initializer cannot mutate the store, so the UUID remains
    /// unique between that check and publication.
    pub fn restore_initialized<F>(
        &mut self,
        uuid: Uuid,
        entity_type_id: i32,
        position: Vec3d,
        data: EntityData,
        initialize: F,
    ) -> Option<i32>
    where
        F: FnOnce(&mut Entity),
    {
        match self.restore_initialized_with_removed_admission(
            uuid,
            entity_type_id,
            position,
            data,
            (false, &mut 0),
            |entity| {
                initialize(entity);
                true
            },
        )? {
            EntityRestoreOutcome::Published(id) => Some(id),
            EntityRestoreOutcome::RejectedRemoved | EntityRestoreOutcome::RejectedDuplicateUuid => {
                None
            }
        }
    }

    pub(crate) fn restore_initialized_with_removed_admission<F>(
        &mut self,
        uuid: Uuid,
        entity_type_id: i32,
        position: Vec3d,
        data: EntityData,
        admission: (bool, &mut usize),
        initialize: F,
    ) -> Option<EntityRestoreOutcome>
    where
        F: FnOnce(&mut Entity) -> bool,
    {
        self.restore_initialized_with_effects(
            uuid,
            entity_type_id,
            position,
            data,
            admission,
            |entity, _| initialize(entity),
        )
    }

    pub(crate) fn restore_initialized_with_effects<F>(
        &mut self,
        uuid: Uuid,
        entity_type_id: i32,
        position: Vec3d,
        data: EntityData,
        (reject_removed, allocations): (bool, &mut usize),
        initialize: F,
    ) -> Option<EntityRestoreOutcome>
    where
        F: FnOnce(&mut Entity, &mut Self) -> bool,
    {
        let expected_resource_id = vanilla_entity_type(entity_type_id)
            .map(|catalog| catalog.resource_id)
            .or_else(|| {
                self.plugins
                    .content_registry()
                    .entity_by_id(entity_type_id)
                    .map(|spec| spec.resource_id_static())
            })?;
        let family_matches = match &data {
            EntityData::Boat(_) => super::boat::is_boat_type(entity_type_id),
            EntityData::ChestBoat(_) => super::chest_boat::is_chest_boat_type(entity_type_id),
            _ => true,
        };
        if !family_matches
            || expected_resource_id != data.entity_resource_id()
            || matches!(&data, EntityData::EnderDragonPart)
        {
            return None;
        }
        if !reject_removed && self.ids_by_uuid.contains_key(&uuid) {
            return None;
        }
        let id = self.allocate_id();
        if entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
            // The parts take the next ids; clients number them after the dragon.
            for _ in 0..crate::entity::ender_dragon::PART_COUNT {
                self.allocate_id();
            }
        }
        *allocations += 1;
        let mut entity = Box::new(Entity::new(id, uuid, entity_type_id, position, data));
        // A refused initializer publishes nothing.
        if !initialize(&mut entity, self) {
            return None;
        }
        if reject_removed {
            if entity.removal_reason().is_some() {
                return Some(EntityRestoreOutcome::RejectedRemoved);
            }
            if self.ids_by_uuid.contains_key(&uuid) {
                return Some(EntityRestoreOutcome::RejectedDuplicateUuid);
            }
        }
        if crate::entity::ender_dragon::state(&entity).is_some() {
            let uuids = std::array::from_fn(|_| self.next_uuid());
            crate::entity::ender_dragon::initialize_parts(&mut entity, uuids);
        }
        let mut slot = EntitySlot::default();
        if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
            let mut runtime = MobAiRuntime::for_entity_with_plugins(
                entity_type_id,
                self.random.clone(),
                &self.plugins,
            );
            runtime.target_state_mut().load_target_pending =
                crate::entity::ai::mob::target::anger::read(&entity)
                    .is_some_and(|anger| anger.target.is_some());
            runtime.set_path_debug_capture(self.entity_path_debug_capture);
            runtime.install_fox_target_goals(&entity);
            if let Some(pos) = runtime.refresh_retained_navigation(&entity) {
                self.lifecycle_events
                    .push(EntityLifecycleEvent::BlockDestruction {
                        entity_id: id,
                        pos,
                        stage: -1,
                    });
            }
            slot.ai = Some(runtime);
            self.ai_entity_ids.insert(id);
        }
        self.entities.insert(id, slot);
        self.ids_by_uuid.insert(uuid, id);
        self.admit_entity(entity);
        Some(EntityRestoreOutcome::Published(id))
    }

    pub(crate) fn send_mob_effect_to_passengers(
        &mut self,
        vehicle_id: i32,
        packet: ClientboundPlayPacket,
    ) {
        let recipient_entity_ids = self
            .passengers(vehicle_id)
            .iter()
            .copied()
            .filter(|id| self.is_external_player_passenger(*id))
            .collect::<Vec<_>>();
        if recipient_entity_ids.is_empty() {
            return;
        }
        self.lifecycle_events
            .push(EntityLifecycleEvent::MobEffectChanged {
                recipient_entity_ids,
                packet,
            });
    }

    /// Whether `id` names a body lifted out for its own tick ([`Self::detach`]).
    pub fn is_detached(&self, id: i32) -> bool {
        !self.players.contains_key(&id)
            && self
                .entities
                .get(&id)
                .is_some_and(|slot| slot.body.is_none())
    }

    pub fn get(&self, id: i32) -> Option<&Entity> {
        self.entities.get(&id)?.body.as_deref()
    }

    pub fn get_mut(&mut self, id: i32) -> Option<&mut Entity> {
        if !self
            .entities
            .get(&id)
            .is_some_and(|slot| slot.body.is_some())
        {
            return None;
        }
        self.mark_entity_chunk_dirty(id);
        self.entities.get_mut(&id)?.body.as_deref_mut()
    }

    pub(crate) fn insert_player(
        &mut self,
        id: i32,
        uuid: Uuid,
        mut player: Box<crate::world::PlayerState>,
    ) {
        let section = section_of_block(player.body().block_position());
        player.body_mut().set_filed_section(section);
        self.file_in_section(id, section);
        self.players.insert(id, player);
        self.entities.entry(id).or_default();
        self.players_by_uuid.insert(uuid, id);
    }

    /// Moves an online player to the section of its current position so
    /// section-bounded queries find it there.
    pub(crate) fn refile_player(&mut self, id: i32) {
        let Some(body) = self.players.get_mut(&id).map(|player| player.body_mut()) else {
            return;
        };
        let filed = body.filed_section();
        let section = section_of_block(body.block_position());
        if filed == section {
            return;
        }
        body.set_filed_section(section);
        self.unfile(id, filed);
        self.file_in_section(id, section);
    }

    pub(crate) fn remove_player(&mut self, id: i32) -> Option<Box<crate::world::PlayerState>> {
        let player = self.players.remove(&id)?;
        if let Some(slot) = self.entities.get(&id) {
            if slot.body.is_none() && slot.vehicle.is_none() && slot.passengers.is_empty() {
                self.entities.remove(&id);
            }
        }
        self.unfile(id, player.body().filed_section());
        self.players_by_uuid.remove(&player.body().uuid());
        Some(player)
    }

    pub(crate) fn player(&self, id: i32) -> Option<&crate::world::PlayerState> {
        self.players.get(&id).map(Box::as_ref)
    }

    pub(crate) fn player_mut(&mut self, id: i32) -> Option<&mut crate::world::PlayerState> {
        self.players.get_mut(&id).map(Box::as_mut)
    }

    pub(crate) fn player_by_uuid(&self, uuid: Uuid) -> Option<&crate::world::PlayerState> {
        self.player(*self.players_by_uuid.get(&uuid)?)
    }

    pub(crate) fn players(&self) -> impl Iterator<Item = &crate::world::PlayerState> + Clone {
        self.players.values().map(Box::as_ref)
    }

    /// A player and another stored body, borrowed together.
    pub(crate) fn player_and_entity_mut(
        &mut self,
        player_id: i32,
        entity_id: i32,
    ) -> Option<(&mut crate::world::PlayerState, &mut Entity)> {
        if player_id == entity_id
            || !self.players.contains_key(&player_id)
            || !self
                .entities
                .get(&entity_id)
                .is_some_and(|slot| slot.body.is_some())
        {
            return None;
        }
        self.mark_entity_chunk_dirty(entity_id);
        let player = self.players.get_mut(&player_id).map(Box::as_mut)?;
        let entity = self.entities.get_mut(&entity_id)?.body.as_deref_mut()?;
        Some((player, entity))
    }

    pub(crate) fn players_mut(&mut self) -> impl Iterator<Item = &mut crate::world::PlayerState> {
        self.players.values_mut().map(Box::as_mut)
    }

    /// Every live body, in no particular order.
    pub(crate) fn bodies(&self) -> impl Iterator<Item = &Entity> {
        self.entities
            .values()
            .filter_map(|slot| slot.body.as_deref())
    }

    pub(crate) fn dragons(&self) -> impl Iterator<Item = &Entity> {
        self.dragon_ids
            .iter()
            .filter_map(|id| self.get(*id))
            .filter(|entity| {
                self.section_visibility(entity.filed_section())
                    .is_accessible()
            })
    }

    pub(crate) fn dragon_ids(&self) -> &BTreeSet<i32> {
        &self.dragon_ids
    }

    pub(crate) fn ender_pearl_ids(&self) -> &FxHashSet<i32> {
        &self.ender_pearl_ids
    }

    pub(crate) fn end_crystal_ids(&self) -> &FxHashSet<i32> {
        &self.end_crystal_ids
    }

    fn contains(&self, id: i32) -> bool {
        self.get(id).is_some()
    }

    /// The slot of an id the caller just validated as stored.
    fn slot_mut(&mut self, id: i32) -> &mut EntitySlot {
        self.entities
            .get_mut(&id)
            .expect("a validated entity id has a slot")
    }

    /// `PersistentEntitySectionManager.isLoaded(UUID)` (`:289-291`).
    pub fn contains_uuid(&self, uuid: Uuid) -> bool {
        self.ids_by_uuid.contains_key(&uuid)
    }

    pub fn entity_id_for_uuid(&self, uuid: Uuid) -> Option<i32> {
        self.ids_by_uuid.get(&uuid).copied()
    }

    pub(crate) fn unavailable_registered_uuids(&self) -> impl Iterator<Item = Uuid> + '_ {
        self.ids_by_uuid.iter().filter_map(|(uuid, id)| {
            (!self.contains(*id) && self.visible_entity_id_for_uuid(*uuid).is_some())
                .then_some(*uuid)
        })
    }

    pub fn len(&self) -> usize {
        self.live_body_count
    }

    pub fn is_empty(&self) -> bool {
        self.live_body_count == 0
    }

    pub(crate) fn owned_bobber_id(&self, owner: Uuid) -> Option<i32> {
        let id = *self.bobbers_by_owner.get(&owner)?;
        self.get(id)
            .filter(|entity| {
                matches!(
                    entity.data(),
                    EntityData::FishingBobber(data) if data.owner() == Some(owner) && entity.is_alive()
                )
            })
            .map(Entity::id)
    }

    pub(super) fn index_body_metadata(&mut self, entity: &Entity) {
        match entity.data() {
            EntityData::FishingBobber(bobber) => {
                if let Some(owner) = bobber.owner() {
                    self.bobbers_by_owner.insert(owner, entity.id());
                }
            }
            EntityData::EnderPearl(_) => {
                self.ender_pearl_ids.insert(entity.id());
            }
            EntityData::EndCrystal(_) => {
                self.end_crystal_ids.insert(entity.id());
            }
            _ => {}
        }
    }

    pub(super) fn unindex_body_metadata(&mut self, entity: &Entity) {
        match entity.data() {
            EntityData::FishingBobber(bobber) => {
                if let Some(owner) = bobber.owner() {
                    if self.bobbers_by_owner.get(&owner) == Some(&entity.id()) {
                        self.bobbers_by_owner.remove(&owner);
                    }
                }
            }
            EntityData::EnderPearl(_) => {
                self.ender_pearl_ids.remove(&entity.id());
            }
            EntityData::EndCrystal(_) => {
                self.end_crystal_ids.remove(&entity.id());
            }
            _ => {}
        }
    }

    /// Every live entity id, ascending.
    pub fn ids(&self) -> Vec<i32> {
        let mut ids: Vec<_> = self.bodies().map(Entity::id).collect();
        ids.sort_unstable();
        ids
    }

    /// Every entity id filed in one section, ascending.
    pub fn ids_in_section(&self, section: SectionKey) -> Vec<i32> {
        let mut ids: Vec<i32> = self
            .by_section
            .get(&section)
            .into_iter()
            .flat_map(|ids| ids.iter().copied())
            .filter(|id| self.contains(*id))
            .collect();
        ids.sort_unstable();
        ids
    }

    /// Every entity id filed in any section of one chunk column, ascending.
    ///
    pub fn ids_in_chunk(&self, chunk: ChunkKey) -> Vec<i32> {
        let mut ids: Vec<i32> = self.iter_chunk_section_ids(chunk).collect();
        ids.sort_unstable();
        ids
    }

    pub fn ids_in_box_sections(&self, box_to_test: Aabb) -> SmallIds {
        self.ids_in_box_sections_with_detached(box_to_test, None)
    }

    // Detached ticking bodies retain their section insertion slot.
    pub fn ids_in_box_sections_with_detached(
        &self,
        box_to_test: Aabb,
        detached: Option<i32>,
    ) -> SmallIds {
        self.ids_in_box_sections_where(box_to_test, |id| self.contains(id) || Some(id) == detached)
    }

    fn ids_in_box_sections_where(
        &self,
        box_to_test: Aabb,
        visible: impl Fn(i32) -> bool,
    ) -> SmallIds {
        let mut ids = SmallIds::new();
        self.for_each_box_section_id(box_to_test, |id| {
            if visible(id) {
                ids.push(id);
            }
        });
        ids
    }

    pub fn is_chunk_occupied(&self, chunk: ChunkKey) -> bool {
        self.sections_by_column.get(&chunk).is_some_and(|sections| {
            sections.iter().any(|section| {
                self.by_section
                    .get(section)
                    .is_some_and(|ids| ids.iter().any(|id| self.contains(*id)))
            })
        })
    }

    pub fn iter_occupied_chunks(&self) -> impl Iterator<Item = ChunkKey> + '_ {
        self.sections_by_column
            .iter()
            .filter_map(|(&column, sections)| {
                sections
                    .iter()
                    .any(|section| {
                        self.by_section
                            .get(section)
                            .is_some_and(|ids| ids.iter().any(|id| self.contains(*id)))
                    })
                    .then_some(column)
            })
    }

    /// Every chunk column that currently holds at least one entity, ascending.
    pub fn occupied_chunks(&self) -> Vec<ChunkKey> {
        let mut chunks: Vec<ChunkKey> = self.iter_occupied_chunks().collect();
        chunks.sort_unstable();
        chunks
    }

    /// `Entity.setRemoved(reason)` down to
    /// `PersistentEntitySectionManager.Callback.onRemove` (`:426-448`): the
    /// entity leaves its section, its UUID stops being known, and an empty
    /// section is dropped.
    ///
    pub fn remove(&mut self, id: i32, reason: RemovalReason) -> Option<Box<Entity>> {
        let mut entity = self.detach(id)?;
        self.unlink_entity(id, Some(&mut entity));
        let framed_map_removal = crate::entity::hanging::framed_map_removal(&entity);
        if reason.should_destroy() {
            crate::entity::slime::split_on_removal(&entity, self);
        }
        let removal_effects =
            crate::entity::living::mob_effects::prepare_mob_removed(&mut entity, reason);
        entity.set_removed(reason);
        self.retire_ai(id);
        if reason.should_destroy() {
            self.drop_destroyed_container_cargo(&mut entity);
        }
        self.unfile(id, entity.filed_section());
        self.ids_by_uuid.remove(&entity.uuid());
        if reason.should_destroy() {
            crate::entity::leash::drop_leash_on_destroy(&mut entity, self);
            if let Some(removal) = framed_map_removal {
                self.queue_command(EntityCommand::FramedMapRemoval(removal));
            }
        }
        self.entities.remove(&id);
        self.dragon_ids.remove(&id);
        self.unindex_body_metadata(&entity);
        if let Some(removal_effects) = removal_effects {
            self.queue_command(EntityCommand::MobEffect(
                crate::entity::projectile_action::mob_effect::PendingMobEffectAction::Removed(
                    crate::entity::projectile_action::mob_effect::PendingRemovalMobEffects {
                        source: removal_effects.source,
                        effects: removal_effects.effects,
                        removed_entity: entity.clone(),
                    },
                ),
            ));
        } else {
            self.publish_removed((*entity).clone());
        }
        Some(entity)
    }

    /// Lifts an entity out for the duration of its tick.
    ///
    ///
    /// The UUID stays registered while the entity is detached, so a second
    /// entity cannot claim it mid-tick.
    pub fn detach(&mut self, id: i32) -> Option<Box<Entity>> {
        // Borrowing is not a section transfer; the body keeps its section slot.
        if !self
            .entities
            .get(&id)
            .is_some_and(|slot| slot.body.is_some())
        {
            return None;
        }
        self.mark_entity_chunk_dirty(id);
        let body = self.entities.get_mut(&id)?.body.take()?;
        self.live_body_count -= 1;
        Some(body)
    }

    /// Runs `run` on a live entity lifted out of the store, then reattaches it.
    pub fn with_entity_mut<R>(
        &mut self,
        id: i32,
        run: impl FnOnce(&mut Entity, &mut Self) -> R,
    ) -> Option<R> {
        let mut body = self.detach(id)?;
        let result = run(&mut body, self);
        self.attach(body);
        Some(result)
    }

    /// The other half of [`EntityStore::detach`]: files the entity back into
    /// the section its *current* block position names
    /// (`PersistentEntitySectionManager.Callback.onMove`, `:376-394`), or
    /// completes its removal when the tick set a removal reason.
    pub fn attach(&mut self, entity: Box<Entity>) {
        let _ = self.attach_checked(entity);
    }

    pub(crate) fn attach_checked(&mut self, mut entity: Box<Entity>) -> Option<()> {
        if let Some(reason) = entity.removal_reason() {
            self.mark_chunk_dirty(entity.filed_section().column());
            self.mark_entity_chunk_dirty(entity.id());
            let published = self
                .entities
                .get(&entity.id())
                .is_some_and(|slot| slot.removal_published);
            let mut cargo_published = true;
            self.unfile(entity.id(), entity.filed_section());
            let framed_map_removal = crate::entity::hanging::framed_map_removal(&entity);
            if reason.should_destroy() {
                crate::entity::slime::split_on_removal(&entity, self);
            }
            let removal_effects =
                crate::entity::living::mob_effects::prepare_mob_removed(&mut entity, reason);
            self.retire_ai(entity.id());
            if reason.should_destroy() && !published {
                cargo_published = self.drop_destroyed_container_cargo(&mut entity);
            }
            self.ids_by_uuid.remove(&entity.uuid());
            self.unlink_entity(entity.id(), Some(&mut entity));
            if reason.should_destroy() {
                if !published {
                    crate::entity::leash::drop_leash_on_destroy(&mut entity, self);
                }
                if let Some(removal) = framed_map_removal {
                    self.queue_command(EntityCommand::FramedMapRemoval(removal));
                }
            }
            self.entities.remove(&entity.id());
            self.dragon_ids.remove(&entity.id());
            self.unindex_body_metadata(&entity);
            if let Some(removal_effects) = removal_effects {
                self.queue_command(EntityCommand::MobEffect(
                    crate::entity::projectile_action::mob_effect::PendingMobEffectAction::Removed(
                        crate::entity::projectile_action::mob_effect::PendingRemovalMobEffects {
                            source: removal_effects.source,
                            effects: removal_effects.effects,
                            removed_entity: entity,
                        },
                    ),
                ));
            } else if !published {
                self.publish_removed(*entity);
            }
            return cargo_published.then_some(());
        }
        if crate::entity::ender_dragon::state(&entity).is_some() {
            self.dragon_ids.insert(entity.id());
        }
        let capture_debug = self.entity_path_debug_capture;
        let slot = self.entities.entry(entity.id()).or_default();
        if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
            let entity_id = entity.id();
            let created = slot.ai.is_none();
            let runtime = slot.ai.get_or_insert_with(|| {
                let mut runtime = MobAiRuntime::for_entity_with_plugins(
                    entity.entity_type_id(),
                    self.random.clone(),
                    &self.plugins,
                );
                runtime.set_path_debug_capture(capture_debug);
                runtime
            });
            runtime.install_fox_target_goals(&entity);
            let clear = runtime.refresh_retained_navigation(&entity);
            if created {
                self.ai_entity_ids.insert(entity_id);
            }
            if let Some(pos) = clear {
                self.lifecycle_events
                    .push(EntityLifecycleEvent::BlockDestruction {
                        entity_id: entity.id(),
                        pos,
                        stage: -1,
                    });
            }
        }
        let waypoint_updated = entity.take_waypoint_update_pending();
        let filed = entity.filed_section();
        let section = section_of_block(entity.block_position());
        self.transfer_section(&entity, filed, section);
        entity.set_filed_section(section);
        self.index_body_metadata(&entity);
        let entity_id = entity.id();
        if self.slot_mut(entity_id).body.replace(entity).is_none() {
            self.live_body_count += 1;
        }
        self.mark_chunk_dirty(filed.column());
        if section.column() != filed.column() {
            self.mark_chunk_dirty(section.column());
        }
        self.mark_entity_chunk_dirty(entity_id);
        if waypoint_updated {
            self.lifecycle_events
                .push(EntityLifecycleEvent::WaypointUpdated { entity_id });
        }
        Some(())
    }

    pub fn publish_deferred_removed(&mut self, entity: Box<Entity>) {
        self.publish_removed(*entity);
    }

    fn drop_destroyed_container_cargo(&mut self, entity: &mut Entity) -> bool {
        let slots = crate::entity::chest_boat::take_destroyed_cargo(entity)
            .or_else(|| crate::entity::minecart_container::take_destroyed_cargo(entity));
        let Some(slots) = slots else {
            return true;
        };
        let planned = crate::entity::item::plan_container_drops(self, entity.position(), &slots);
        crate::entity::item::spawn_planned_container_drops(self, planned)
    }

    ///
    /// The section index is keyed on the entity's block position, so every
    /// position write outside a tick has to be followed by this; the entity
    /// remembers which section it was filed under, exactly as vanilla's
    /// callback remembers `currentSectionKey` (`:367`), so the move is exact
    /// rather than a search.
    pub fn refile(&mut self, id: i32) {
        let Some(entity) = self.get_mut(id) else {
            return;
        };
        let filed = entity.filed_section();
        let current = section_of_block(entity.block_position());
        let waypoint_updated = entity.take_waypoint_update_pending();
        if filed != current {
            let previous_visibility = self.section_visibility(filed);
            let current_visibility = self.section_visibility(current);
            self.unfile(id, filed);
            self.file_in_section(id, current);
            self.mark_chunk_dirty(current.column());
            let entity = self
                .entities
                .get_mut(&id)
                .and_then(|slot| slot.body.as_deref_mut())
                .expect("the entity remains live while it is refiled");
            self.visibility.transition_section_visibility(
                &mut self.lifecycle_events,
                entity,
                filed,
                current,
                previous_visibility,
                current_visibility,
            );
            entity.set_filed_section(current);
            if !previous_visibility.is_accessible() && current_visibility.is_accessible() {
                self.publish_existing_passengers(id);
            }
        }
        if waypoint_updated {
            self.lifecycle_events
                .push(EntityLifecycleEvent::WaypointUpdated { entity_id: id });
        }
    }

    pub(super) fn file_in_section(&mut self, id: i32, section: SectionKey) {
        let ids = self.by_section.entry(section).or_default();
        let inserted_section = ids.is_empty();
        ids.insert(id);
        if inserted_section {
            let column_sections = self.sections_by_column.entry(section.column()).or_default();
            if let Err(index) = column_sections.binary_search(&section) {
                column_sections.insert(index, section);
            }
        }
    }

    /// `PersistentEntitySectionManager.removeSectionIfEmpty` (`:55-59`).
    fn unfile(&mut self, id: i32, section: SectionKey) {
        if let Some(ids) = self.by_section.get_mut(&section) {
            ids.remove(&id);
            if ids.is_empty() {
                self.by_section.remove(&section);
                let column = section.column();
                if let Some(column_sections) = self.sections_by_column.get_mut(&column) {
                    if let Ok(index) = column_sections.binary_search(&section) {
                        column_sections.remove(index);
                    }
                    if column_sections.is_empty() {
                        self.sections_by_column.remove(&column);
                    }
                }
            }
        }
    }
}

impl EntityStore {
    pub fn leash_entity_to(&mut self, entity_id: i32, holder_id: i32, synch: bool) -> bool {
        if entity_id == holder_id
            || self
                .get(holder_id)
                .is_none_or(|holder| !crate::entity::leash::alive_for_leash(holder))
            || self.get(entity_id).is_none_or(|entity| {
                !crate::entity::leash::alive_for_leash(entity)
                    || !crate::entity::leash::supports_leash(entity)
            })
            || self.vehicle(entity_id).is_some()
        {
            return false;
        }
        let mut entity = self.detach(entity_id).expect("validated leashee");
        let result = crate::entity::leash::set_leashed_to(&mut entity, self, holder_id, synch);
        if self.slot_mut(entity_id).body.replace(entity).is_none() {
            self.live_body_count += 1;
        }
        result
    }
}

mod mob_control;
pub mod teleport;
