//! EntityReference identity for living references and Item thrower caches.

use crate::world::simulation_level::LevelCtx;
use rustc_hash::FxHashMap;
use std::hash::BuildHasherDefault;

use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID;
use uuid::Uuid;

use crate::entity::store::EntityStore;
use crate::entity::Entity;

///
/// `EntityReference` is an `Either<UUID, StoredEntityType>` whose right arm
/// holds the entity object itself, and `getUUID` (`:51-53`) answers off that
/// object even after the level removed it -- which is why the UUID is part of
/// the arm and not looked up again at save time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityReference {
    /// `Either.left(uuid)` (`:38-40`): the UUID alone, resolved through the
    /// level's lookup on every read (`:55-76`). A session player is the entity
    /// this arm normally names, because no store holds one, and a reloaded
    /// record reaches it for any entity.
    Uuid(Uuid),
    /// `Either.right(entity)` (`:34-36`): the stored entity the level's own
    /// lookup files under `entity_id`, beside the UUID it answers.
    ///
    /// The UUID is absent only for a caller that named the entity by its id
    /// alone and had no store at hand to read it off; a reference created from
    /// a stored entity carries it, so a target the store has already dropped
    /// still saves the UUID Vanilla's object holds.
    Stored { entity_id: i32, uuid: Option<Uuid> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceTarget {
    /// `Entity.getType()` as the built-in entity type id.
    pub entity_type_id: i32,
    /// `LivingEntity.isAlive()` (`:1718-1720`): `!isRemoved() && health > 0`.
    pub alive: bool,
}

/// Remote online-player facts for a level frame. Stored entities are borrowed
/// from their owning level only when a reference actually queries them.
#[derive(Debug, Clone, Default)]
pub struct CrossLevelEntities {
    players: FxHashMap<Uuid, crate::entity::level::NearbyPlayer>,
    player_ids: FxHashMap<i32, Uuid>,
}

impl CrossLevelEntities {
    pub const EMPTY: Self = Self {
        players: FxHashMap::with_hasher(BuildHasherDefault::new()),
        player_ids: FxHashMap::with_hasher(BuildHasherDefault::new()),
    };

    pub(crate) fn clear(&mut self) {
        self.players.clear();
        self.player_ids.clear();
    }

    pub fn insert_session_input(&mut self, player: crate::entity::level::NearbyPlayer) {
        if let Some(old) = self.players.insert(player.uuid, player) {
            self.player_ids.remove(&old.entity_id);
        }
        self.player_ids.insert(player.entity_id, player.uuid);
    }

    pub fn target_input(&self, uuid: Uuid) -> Option<&crate::entity::level::NearbyPlayer> {
        self.players.get(&uuid)
    }

    pub fn target_input_by_id(
        &self,
        entity_id: i32,
    ) -> Option<&crate::entity::level::NearbyPlayer> {
        self.target_input(*self.player_ids.get(&entity_id)?)
    }

    pub fn player_entity_id(&self, uuid: Uuid) -> Option<i32> {
        self.target_input(uuid).map(|player| player.entity_id)
    }

    pub fn get(&self, uuid: Uuid) -> Option<ReferenceTarget> {
        self.target_input(uuid).map(|player| ReferenceTarget {
            entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
            alive: !player.dead_or_dying,
        })
    }
}

pub type LastHurtByMob = EntityReference;

impl EntityReference {
    /// `EntityReference.getUUID()` (`EntityReference.java:51-53`), `None` for
    /// the one arm whose caller had no entity to read a UUID off.
    #[must_use]
    pub const fn uuid(self) -> Option<Uuid> {
        match self {
            Self::Uuid(uuid) => Some(uuid),
            Self::Stored { uuid, .. } => uuid,
        }
    }

    /// The id this level's store files the held entity under
    /// (`Either.right`, `:56-58`), which is the reference's own fast path.
    #[must_use]
    pub const fn stored_entity_id(self) -> Option<i32> {
        match self {
            Self::Uuid(_) => None,
            Self::Stored { entity_id, .. } => Some(entity_id),
        }
    }

    /// The stored entity this level's own store files under the reference's
    /// UUID (`:66-73`), filtered to a living entity the way
    /// `EntityReference.resolve` (`:84-86`) filters the class `getLastHurtByMob`
    /// asks for.
    ///
    /// This is the stored half of the lookup, shared with the warm-up a
    /// restored reference runs so the two cannot disagree about which entity
    /// the reference names.
    #[must_use]
    pub fn stored_uuid_target(self, store: &EntityStore) -> Option<i32> {
        let uuid = self.uuid()?;
        store.visible_entity_id_for_uuid(uuid).filter(|id| {
            store
                .get(*id)
                .is_some_and(|entity| entity.living().is_some())
        })
    }

    ///
    /// A session player is a stored entity of its own level in Vanilla
    /// (`ServerLevel.addPlayer`, `:969-978`), so this runtime answers one from
    /// the level it ticks against and from the projection for the levels it
    /// cannot borrow. `None` is the null Vanilla answers for a UUID no level
    /// holds, an entity the class refuses, or a removed one.
    #[must_use]
    pub(crate) fn resolve(
        self,
        store: &EntityStore,
        level: &LevelCtx<'_, '_>,
    ) -> Option<ReferenceTarget> {
        // `:56-64`: the entity the right arm holds wins, and a removed one
        // falls through to the UUID lookup below.
        if let Some(entity_id) = self.stored_entity_id() {
            if let Some(entity) = store.get(entity_id) {
                return ReferenceTarget::of_entity(entity);
            }
        }
        let uuid = self.uuid()?;
        if let Some(entity_id) = store.visible_entity_id_for_uuid(uuid) {
            if let Some(entity) = store.get(entity_id) {
                return ReferenceTarget::of_entity(entity);
            }
        }
        // The same manager's sessions, which `EntityReference.getPlayer`
        // (`:116-118`) reaches off the level.
        if let Some(player) = level.player(uuid) {
            return Some(ReferenceTarget {
                entity_type_id: VANILLA_ENTITY_TYPE_PLAYER_ID,
                alive: !player.dead_or_dying,
            });
        }
        // `:1384-1391`: every level beside this one, their sessions included.
        level.entity_in_other_dimension(uuid)
    }
}

impl ReferenceTarget {
    pub(crate) fn of_entity(entity: &Entity) -> Option<Self> {
        let living = entity.living()?;
        Some(Self {
            entity_type_id: entity.entity_type_id(),
            alive: entity.is_alive() && living.health() > 0.0,
        })
    }
}
