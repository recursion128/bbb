pub(crate) mod pufferfish;

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::builtin_enum_component_values::{
    normalize_vanilla_dye_color_id, VANILLA_DYE_COLOR_VALUES, VANILLA_SALMON_VARIANT_LARGE_ID,
    VANILLA_SALMON_VARIANT_MEDIUM_ID, VANILLA_SALMON_VARIANT_SMALL_ID,
    VANILLA_SALMON_VARIANT_VALUES,
};
use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_INT_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID, VANILLA_SALMON_VARIANT_DATA_ID,
    VANILLA_TROPICAL_FISH_VARIANT_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_PUFFERFISH_ID, VANILLA_ENTITY_TYPE_SALMON_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
};
use bbb_protocol::packets::{EntityDataValue, EntityDataValueKind, Vec3d};
use bbb_protocol::tropical_fish_patterns::{
    vanilla_tropical_fish_pattern, VANILLA_TROPICAL_FISH_PATTERNS,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use crate::entity::living::SynchedValue;
use crate::entity::save::optional_field;
use crate::entity::save::optional_int;
use crate::entity::{Entity, EntityData, EntityStore, GeneralLivingData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchoolingFishState {
    leader_uuid: Option<Uuid>,
    school_size: i32,
}

impl SchoolingFishState {
    pub(crate) const fn new() -> Self {
        Self {
            leader_uuid: None,
            school_size: 1,
        }
    }

    pub const fn leader_uuid(self) -> Option<Uuid> {
        self.leader_uuid
    }

    pub const fn school_size(self) -> i32 {
        self.school_size
    }

    pub(crate) fn set_leader_uuid(&mut self, leader_uuid: Option<Uuid>) {
        self.leader_uuid = leader_uuid;
    }

    pub(crate) fn increment_school_size(&mut self) {
        self.school_size = self.school_size.wrapping_add(1);
    }
}

pub(crate) fn uses_fish_move_control(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_COD_ID
            | VANILLA_ENTITY_TYPE_PUFFERFISH_ID
            | VANILLA_ENTITY_TYPE_SALMON_ID
            | VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID
    )
}

pub(crate) fn schooling_fish_state_mut(body: &mut Entity) -> Option<&mut SchoolingFishState> {
    let EntityData::GeneralLiving(data) = &mut body.data else {
        return None;
    };
    data.fish_state_mut()?.school.as_mut()
}

fn live_school_leader<'a>(body: &Entity, store: &'a EntityStore) -> Option<&'a Entity> {
    let uuid = crate::entity::schooling_fish_state(body)?.leader_uuid()?;
    let leader = store.get(store.entity_id_for_uuid(uuid)?)?;
    (leader.entity_type_id() == body.entity_type_id()
        && leader.is_alive()
        && leader.living().is_some_and(|living| living.health() > 0.0))
    .then_some(leader)
}

pub(crate) fn can_random_swim(body: &Entity, store: &EntityStore) -> bool {
    live_school_leader(body, store).is_none()
}

fn stop_following(body: &mut Entity, store: &mut EntityStore) {
    let state = schooling_fish_state_mut(body).expect("schooling body");
    let Some(uuid) = state.leader_uuid.take() else {
        return;
    };
    if let Some(leader) = store
        .entity_id_for_uuid(uuid)
        .and_then(|id| store.get_mut(id))
    {
        if let Some(state) = schooling_fish_state_mut(leader) {
            state.school_size = state.school_size.saturating_sub(1).max(1);
        }
    }
}

pub(crate) fn tick_schooling(body: &mut Entity, store: &mut EntityStore, level: &LevelCtx<'_, '_>) {
    let Some(state) = crate::entity::schooling_fish_state(body) else {
        return;
    };
    if !store
        .mob_ai(body.id())
        .is_some_and(|runtime| runtime.can_follow_school())
    {
        if state.leader_uuid().is_some() {
            stop_following(body, store);
        }
        return;
    }
    if state.leader_uuid().is_some() {
        let leader = live_school_leader(body, store).map(Entity::position);
        // AbstractSchoolingFish.java:85-91.
        if let Some(position) =
            leader.filter(|position| body.position().distance_to_sqr(*position) <= 121.0)
        {
            if body.tick_count() % 10 == 0 {
                let Some(malus) = store.effective_pathfinding_malus(body, level) else {
                    return;
                };
                let Some(mob) = crate::entity::ai::mob::navigation::MobPathingSnapshot::from_entity(
                    body,
                    false,
                    store.vehicle(body.id()).is_some(),
                    store.difficulty(),
                    malus,
                ) else {
                    return;
                };
                let target = bbb_protocol::packets::BlockPos {
                    x: position.x.floor() as i32,
                    y: position.y.floor() as i32,
                    z: position.z.floor() as i32,
                };
                if let Some(runtime) = store.mob_ai_mut(body.id()) {
                    runtime.navigate_to(level, mob, target, 1, 1.0);
                }
            }
            return;
        }
        stop_following(body, store);
    }
    if body.tick_count() % 100 != 0 {
        return;
    }
    let reach = body.bounding_box().inflate(8.0, 8.0, 8.0);
    let neighbors: Vec<_> = store
        .ids_in_box_sections(reach)
        .into_iter()
        .filter(|&id| {
            store.get(id).is_some_and(|other| {
                other.entity_type_id() == body.entity_type_id()
                    && other.is_alive()
                    && other.living().is_some_and(|living| living.health() > 0.0)
                    && other.bounding_box().intersects(reach)
            })
        })
        .collect();
    let current = crate::entity::schooling_fish_state(body).expect("schooling body");
    if current.school_size() > 1 {
        if neighbors.is_empty() {
            schooling_fish_state_mut(body).unwrap().school_size = 1;
        }
        return;
    }
    let Ok(max_size) = crate::entity::natural_spawn::mob_eligibility::natural_spawn_cluster_size(
        body.entity_type_id(),
    ) else {
        return;
    };
    let leader_id = neighbors
        .iter()
        .copied()
        .find(|&id| {
            store
                .get(id)
                .and_then(crate::entity::schooling_fish_state)
                .is_some_and(|state| {
                    state.leader_uuid().is_none()
                        && state.school_size() > 1
                        && state.school_size() < max_size
                })
        })
        .unwrap_or(body.id());
    let (leader_uuid, size) = if leader_id == body.id() {
        (body.uuid(), current.school_size())
    } else {
        let leader = store.get(leader_id).unwrap();
        (
            leader.uuid(),
            crate::entity::schooling_fish_state(leader)
                .unwrap()
                .school_size(),
        )
    };
    let mut added = 0;
    for id in std::iter::once(body.id()).chain(neighbors) {
        if id == leader_id || size + added >= max_size {
            continue;
        }
        if id == body.id() {
            schooling_fish_state_mut(body).unwrap().leader_uuid = Some(leader_uuid);
            added += 1;
        } else if store.get(id).is_some_and(|fish| {
            can_random_swim(fish, store)
                && store
                    .mob_ai(id)
                    .is_some_and(|runtime| runtime.can_follow_school())
                && crate::entity::schooling_fish_state(fish)
                    .is_some_and(|state| state.school_size() == 1)
        }) {
            schooling_fish_state_mut(store.get_mut(id).unwrap())
                .unwrap()
                .leader_uuid = Some(leader_uuid);
            added += 1;
        }
    }
    let leader = if leader_id == body.id() {
        body
    } else {
        store.get_mut(leader_id).unwrap()
    };
    schooling_fish_state_mut(leader).unwrap().school_size += added;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalmonVariant {
    Small,
    Medium,
    Large,
}

impl SalmonVariant {
    pub const fn id(self) -> i32 {
        match self {
            Self::Small => VANILLA_SALMON_VARIANT_SMALL_ID,
            Self::Medium => VANILLA_SALMON_VARIANT_MEDIUM_ID,
            Self::Large => VANILLA_SALMON_VARIANT_LARGE_ID,
        }
    }

    pub fn serialized_name(self) -> &'static str {
        VANILLA_SALMON_VARIANT_VALUES
            .iter()
            .find(|entry| entry.id == self.id())
            .expect("generated Salmon variant")
            .serialized_name
    }

    pub(crate) fn from_name(name: &str) -> Option<Self> {
        let id = VANILLA_SALMON_VARIANT_VALUES
            .iter()
            .find(|entry| entry.serialized_name == name)?
            .id;
        match id {
            VANILLA_SALMON_VARIANT_SMALL_ID => Some(Self::Small),
            VANILLA_SALMON_VARIANT_MEDIUM_ID => Some(Self::Medium),
            VANILLA_SALMON_VARIANT_LARGE_ID => Some(Self::Large),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TropicalFishVariant {
    packed: i32,
}

impl TropicalFishVariant {
    pub fn new(pattern_packed_id: i32, base_color_id: i32, pattern_color_id: i32) -> Option<Self> {
        VANILLA_TROPICAL_FISH_PATTERNS
            .iter()
            .any(|entry| entry.packed_id == pattern_packed_id)
            .then_some(())?;
        VANILLA_DYE_COLOR_VALUES
            .iter()
            .any(|entry| entry.id == base_color_id)
            .then_some(())?;
        VANILLA_DYE_COLOR_VALUES
            .iter()
            .any(|entry| entry.id == pattern_color_id)
            .then_some(())?;
        Some(Self {
            packed: pack_tropical_variant(pattern_packed_id, base_color_id, pattern_color_id),
        })
    }

    pub const fn packed(self) -> i32 {
        self.packed
    }

    fn from_saved(packed: i32) -> Self {
        let pattern = vanilla_tropical_fish_pattern(packed & 0xffff).packed_id;
        let base_color = normalize_vanilla_dye_color_id((packed >> 16) & 0xff);
        let pattern_color = normalize_vanilla_dye_color_id((packed >> 24) & 0xff);
        Self {
            packed: pack_tropical_variant(pattern, base_color, pattern_color),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishSpawnKind {
    Cod,
    Salmon(SalmonVariant),
    Tropical(TropicalFishVariant),
}

impl FishSpawnKind {
    const fn entity_type_id(self) -> i32 {
        match self {
            Self::Cod => VANILLA_ENTITY_TYPE_COD_ID,
            Self::Salmon(_) => VANILLA_ENTITY_TYPE_SALMON_ID,
            Self::Tropical(_) => VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FishLeafState {
    Pufferfish(i32),
    Salmon(SalmonVariant),
    Tropical(TropicalFishVariant),
}

impl FishLeafState {
    pub(crate) const fn new(entity_type_id: i32) -> Option<Self> {
        match entity_type_id {
            VANILLA_ENTITY_TYPE_PUFFERFISH_ID => Some(Self::Pufferfish(0)),
            VANILLA_ENTITY_TYPE_SALMON_ID => Some(Self::Salmon(SalmonVariant::Medium)),
            VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => {
                Some(Self::Tropical(TropicalFishVariant { packed: 0 }))
            }
            _ => None,
        }
    }
}

impl EntityStore {
    pub fn spawn_fish(&mut self, position: Vec3d, y_rot: f32, kind: FishSpawnKind) -> Option<i32> {
        let entity_type_id = kind.entity_type_id();
        let mut data = GeneralLivingData::new(entity_type_id)?;
        if !initialize_spawn(&mut data, kind) {
            return None;
        }
        let uuid = self.next_uuid();
        self.restore_initialized(
            uuid,
            entity_type_id,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_rot(y_rot, 0.0);
                entity
                    .living_mut()
                    .expect("constructed fish is living")
                    .y_head_rot = y_rot;
            },
        )
    }
}

fn initialize_spawn(data: &mut GeneralLivingData, kind: FishSpawnKind) -> bool {
    match (
        kind,
        data.fish_state_mut().and_then(|fish| fish.leaf.as_mut()),
    ) {
        (FishSpawnKind::Cod, None) => true,
        (FishSpawnKind::Salmon(variant), Some(FishLeafState::Salmon(state))) => {
            *state = variant;
            true
        }
        (FishSpawnKind::Tropical(variant), Some(FishLeafState::Tropical(state))) => {
            *state = variant;
            true
        }
        _ => false,
    }
}

pub(crate) fn set_spawn_salmon_variant(
    entity: &mut crate::entity::Entity,
    variant: SalmonVariant,
) -> bool {
    set_spawn_leaf(entity, FishLeafState::Salmon(variant))
}

pub(crate) fn set_spawn_tropical_variant(
    entity: &mut crate::entity::Entity,
    variant: TropicalFishVariant,
) -> bool {
    set_spawn_leaf(entity, FishLeafState::Tropical(variant))
}

pub(crate) fn set_puff_state(entity: &mut crate::entity::Entity, state: i32) -> bool {
    set_spawn_leaf(entity, FishLeafState::Pufferfish(state))
}

fn set_spawn_leaf(entity: &mut crate::entity::Entity, next: FishLeafState) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    let Some(current) = data.fish_leaf() else {
        return false;
    };
    if !matches!(
        (current, next),
        (FishLeafState::Pufferfish(_), FishLeafState::Pufferfish(_))
            | (FishLeafState::Salmon(_), FishLeafState::Salmon(_))
            | (FishLeafState::Tropical(_), FishLeafState::Tropical(_))
    ) {
        return false;
    }
    if current != next {
        data.fish_state_mut().expect("a fish leaf").leaf = Some(next);
        entity.mark_synched_dirty(match next {
            FishLeafState::Pufferfish(_) => VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID,
            FishLeafState::Salmon(_) => VANILLA_SALMON_VARIANT_DATA_ID,
            FishLeafState::Tropical(_) => VANILLA_TROPICAL_FISH_VARIANT_DATA_ID,
        });
        if matches!(
            next,
            FishLeafState::Salmon(_) | FishLeafState::Pufferfish(_)
        ) {
            crate::entity::living::refresh_dimensions(entity);
        }
    }
    true
}

pub(crate) fn finish_load(
    entity: &mut crate::entity::Entity,
    previous: Option<&crate::entity::Entity>,
    level: Option<&LevelCtx<'_, '_>>,
) {
    let state = match entity.data() {
        EntityData::GeneralLiving(data) => data.fish_leaf(),
        _ => None,
    };
    let previous = previous
        .and_then(|entity| match entity.data() {
            EntityData::GeneralLiving(data) => data.fish_leaf(),
            _ => None,
        })
        .or_else(|| FishLeafState::new(entity.entity_type_id()));
    if state != previous
        && matches!(
            state,
            Some(FishLeafState::Salmon(_) | FishLeafState::Pufferfish(_))
        )
    {
        let _ = crate::entity::living::refresh_dimensions_with_context(entity, level, None);
    }
}

pub(crate) fn school_follow_supported(
    store: &EntityStore,
    follower: &crate::entity::Entity,
    leader_uuid: Uuid,
) -> bool {
    let Some(follower_state) = crate::entity::schooling_fish_state(follower) else {
        return false;
    };
    let Some(leader_id) = store.entity_id_for_uuid(leader_uuid) else {
        return false;
    };
    let Some(leader) = store.get(leader_id) else {
        return false;
    };
    let Some(leader_state) = crate::entity::schooling_fish_state(leader) else {
        return false;
    };
    follower.uuid() != leader_uuid
        && follower.entity_type_id() == leader.entity_type_id()
        && follower_state.leader_uuid() == Some(leader_uuid)
        && follower_state.school_size() == 1
        && leader_state.leader_uuid().is_none()
}

pub(crate) fn commit_school_follow(
    store: &mut EntityStore,
    follower_id: i32,
    leader_uuid: Uuid,
) -> bool {
    let supported = store
        .get(follower_id)
        .is_some_and(|follower| school_follow_supported(store, follower, leader_uuid));
    if !supported {
        return false;
    }
    let Some(leader_id) = store.entity_id_for_uuid(leader_uuid) else {
        return false;
    };
    crate::entity::general_living::increment_schooling_fish_size(
        store
            .get_mut(leader_id)
            .expect("validated schooling leader remains attached"),
    )
}

pub(crate) fn owns_save_field(name: &str) -> bool {
    matches!(name, "type" | "Variant" | "PuffState")
}

pub(crate) fn write_save_data(entries: &mut Vec<NbtSummaryEntry>, state: FishLeafState) {
    match state {
        FishLeafState::Pufferfish(state) => entries.push(int_entry("PuffState", state)),
        FishLeafState::Salmon(variant) => {
            entries.push(string_entry("type", variant.serialized_name()));
        }
        FishLeafState::Tropical(variant) => {
            entries.push(int_entry("Variant", variant.packed()));
        }
    }
}

pub(crate) fn read_save_data(
    entity_type_id: i32,
    entries: &[NbtSummaryEntry],
) -> Option<FishLeafState> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_PUFFERFISH_ID => Some(FishLeafState::Pufferfish(
            optional_int(entries, "PuffState").unwrap_or(0).min(2),
        )),
        VANILLA_ENTITY_TYPE_SALMON_ID => Some(FishLeafState::Salmon(
            match optional_field(entries, "type") {
                Some(NbtSummaryValue::String(name)) => name
                    .with_utf8(SalmonVariant::from_name)
                    .flatten()
                    .unwrap_or(SalmonVariant::Medium),
                _ => SalmonVariant::Medium,
            },
        )),
        VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID => Some(FishLeafState::Tropical(
            TropicalFishVariant::from_saved(optional_int(entries, "Variant").unwrap_or(0)),
        )),
        _ => None,
    }
}

pub(crate) fn synched_values(state: FishLeafState) -> [SynchedValue; 1] {
    let (data_id, value, default) = match state {
        FishLeafState::Pufferfish(state) => (VANILLA_PUFFERFISH_PUFF_STATE_DATA_ID, state, 0),
        FishLeafState::Salmon(variant) => (
            VANILLA_SALMON_VARIANT_DATA_ID,
            variant.id(),
            VANILLA_SALMON_VARIANT_MEDIUM_ID,
        ),
        FishLeafState::Tropical(variant) => {
            (VANILLA_TROPICAL_FISH_VARIANT_DATA_ID, variant.packed(), 0)
        }
    };
    [SynchedValue {
        value: EntityDataValue {
            data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(value),
        },
        is_default: value == default,
    }]
}

const fn pack_tropical_variant(pattern: i32, base_color: i32, pattern_color: i32) -> i32 {
    pattern & 0xffff | (base_color & 0xff) << 16 | (pattern_color & 0xff) << 24
}

fn int_entry(name: &str, value: i32) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::Int(value),
    }
}

fn string_entry(name: &str, value: &str) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value: NbtSummaryValue::string(value),
    }
}

#[cfg(test)]
#[path = "fish/tests.rs"]
mod tests;
