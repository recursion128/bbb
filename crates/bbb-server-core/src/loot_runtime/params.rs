//! `LootContextParamSets` and the `LootParams` bundle one of them binds.
//!
//! [`super`] evaluates a loot table; this module is the *interface* it
//! evaluates it against. Vanilla splits the same way: `LootParams`
//! (`LootParams.java:14-25`) collects values, `LootParams.Builder.create`
//! (`LootParams.java:92-95`) binds them to a `ContextKeySet` through
//! `ContextMap.Builder.create` (`ContextMap.java:71-83`), and a `LootContext`
//! (`LootContext.java:35-45`) is what a condition, function or entry reads.
//!
//! # The parameter sets
//!
//! `LootContextParamSets` (`LootContextParamSets.java:21-140`) registers 26
//! sets over the 15 keys `LootContextParams` declares
//! (`LootContextParams.java:14-28`), required and optional kept apart the way
//! `ContextKeySet` keeps them (`ContextKeySet.java:11-14`). That registry is
//! not written here: [`LOOT_CONTEXT_PARAM_SETS`] and [`LootContextParamSet`]
//! are the generated `bbb_registry_codec::loot_context_types` catalog, and
//! [`param_sets`] names its entries by the constant each one is registered
//! under.
//!
//! # What this crate can put in one
//!
//! Damage sources carry actual values and their generation tag authority.
//! Entity slots retain the projections each caller can provide.
//!
//! # Where a parameter set failure surfaces
//!
//! Two different vanilla mechanisms, kept apart here because they behave
//! differently:
//!
//! * `ContextMap.Builder.create` (`ContextMap.java:71-83`) throws when the
//!   *caller* offers a key the set does not allow, or omits one it requires.
//!   [`LootParams::create`] is that check.
//! * `ValidationContext.validateContextUsage` (`ValidationContext.java:68-74`)
//!   only *reports* when a *node* names a key — through
//!   `getReferencedContextParams` — the set does not allow. It is a load-time
//!   report and the table still rolls, so it lives in [`super`]'s validation
//!   walk alone; every read below answers a key the set lacks as absent, which
//!   is what `context.getOptionalParameter` answers.
//!
//! Whether a node names the key it reads therefore changes nothing here, only
//! which walk reports a problem: `ExplosionCondition` names `EXPLOSION_RADIUS`
//! (`ExplosionCondition.java:23-25`) and `ApplyExplosionDecay` names nothing
//! (`LootContextUser.java:7-9`), yet both take the same null branch.

use bbb_protocol::block_state_types::{vanilla_block_state_identity, VanillaBlockStateIdentity};
use bbb_protocol::packets::BlockPos;

use super::{LootBlockStateAt, LootEntity, LootRuntimeReason, LootTool};
use crate::block_entity::{BlockEntity, BlockEntityItemProjection, CollectedComponents};

/// The keys, the sets and the entity targets are the generated catalog's, under
/// this module's own names: `bbb_registry_codec::loot_context_types` is derived
/// from `LootContextParams.java`, `LootContextParamSets.java` and
/// `LootContext.java` by the bootstrap generator, and a second hand-written copy
/// would be a second thing to keep in step with 26.1.
pub use bbb_registry_codec::loot_context_types::{
    VanillaLootContextParam as LootContextParam, VanillaLootContextParamSet as LootContextParamSet,
    VANILLA_LOOT_CONTEXT_PARAMS as LOOT_CONTEXT_PARAMS,
    VANILLA_LOOT_CONTEXT_PARAM_SETS as LOOT_CONTEXT_PARAM_SETS,
};

use LootContextParam as Param;

/// The 26 sets by the name `LootContextParamSets` gives each one
/// (`LootContextParamSets.java:21-140`).
pub mod param_sets {
    use super::{LootContextParamSet, LOOT_CONTEXT_PARAM_SETS};

    /// The generated entry for one `LootContextParamSets` constant, resolved at
    /// compile time: a name the catalog does not register is a build error
    /// rather than a `None` at run time.
    const fn named(constant_name: &str) -> &'static LootContextParamSet {
        let mut index = 0;
        while index < LOOT_CONTEXT_PARAM_SETS.len() {
            if str_eq(LOOT_CONTEXT_PARAM_SETS[index].constant_name, constant_name) {
                return &LOOT_CONTEXT_PARAM_SETS[index];
            }
            index += 1;
        }
        panic!("no LootContextParamSets constant by that name")
    }

    const fn str_eq(left: &str, right: &str) -> bool {
        let (left, right) = (left.as_bytes(), right.as_bytes());
        if left.len() != right.len() {
            return false;
        }
        let mut index = 0;
        while index < left.len() {
            if left[index] != right[index] {
                return false;
            }
            index += 1;
        }
        true
    }

    pub const EMPTY: &LootContextParamSet = named("EMPTY");
    pub const CHEST: &LootContextParamSet = named("CHEST");
    pub const COMMAND: &LootContextParamSet = named("COMMAND");
    pub const SELECTOR: &LootContextParamSet = named("SELECTOR");
    pub const VILLAGER_TRADE: &LootContextParamSet = named("VILLAGER_TRADE");
    pub const FISHING: &LootContextParamSet = named("FISHING");
    pub const ENTITY: &LootContextParamSet = named("ENTITY");
    pub const EQUIPMENT: &LootContextParamSet = named("EQUIPMENT");
    pub const ARCHAEOLOGY: &LootContextParamSet = named("ARCHAEOLOGY");
    pub const GIFT: &LootContextParamSet = named("GIFT");
    /// Registered as `barter` rather than after its constant.
    pub const PIGLIN_BARTER: &LootContextParamSet = named("PIGLIN_BARTER");
    pub const VAULT: &LootContextParamSet = named("VAULT");
    pub const ADVANCEMENT_REWARD: &LootContextParamSet = named("ADVANCEMENT_REWARD");
    pub const ADVANCEMENT_ENTITY: &LootContextParamSet = named("ADVANCEMENT_ENTITY");
    pub const ADVANCEMENT_LOCATION: &LootContextParamSet = named("ADVANCEMENT_LOCATION");
    pub const BLOCK_USE: &LootContextParamSet = named("BLOCK_USE");
    /// Registered as `generic`, and the default a table's `type` field falls
    /// back to (`LootTable.java:35`).
    pub const ALL_PARAMS: &LootContextParamSet = named("ALL_PARAMS");
    pub const BLOCK: &LootContextParamSet = named("BLOCK");
    pub const SHEARING: &LootContextParamSet = named("SHEARING");
    pub const ENTITY_INTERACT: &LootContextParamSet = named("ENTITY_INTERACT");
    pub const BLOCK_INTERACT: &LootContextParamSet = named("BLOCK_INTERACT");
    pub const ENCHANTED_DAMAGE: &LootContextParamSet = named("ENCHANTED_DAMAGE");
    pub const ENCHANTED_ITEM: &LootContextParamSet = named("ENCHANTED_ITEM");
    pub const ENCHANTED_LOCATION: &LootContextParamSet = named("ENCHANTED_LOCATION");
    pub const ENCHANTED_ENTITY: &LootContextParamSet = named("ENCHANTED_ENTITY");
    pub const HIT_BLOCK: &LootContextParamSet = named("HIT_BLOCK");
}

/// `LootContextParams.ORIGIN` (`LootContextParams.java:21`), a `Vec3`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LootOrigin {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl LootOrigin {
    /// `Vec3.atCenterOf(pos)` (`Vec3.java:53-55`) over
    /// `Vec3.atLowerCornerWithOffset` (`Vec3.java:49-51`): the origin both
    /// `Block.getDrops` overloads fill (`Block.java:365-371`, `Block.java:373-386`).
    #[must_use]
    pub fn at_center_of(pos: BlockPos) -> Self {
        Self {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        }
    }

    /// `BlockPos.containing` (`BlockPos.java:91-93`) of this origin offset by an
    /// integral vector, which is the position `LocationCheck.test`
    /// (`LocationCheck.java:35-47`) reaches through
    /// `LocationPredicate.matches` (`LocationPredicate.java:44-53`).
    #[must_use]
    pub fn offset_containing(self, offset: BlockPos) -> BlockPos {
        BlockPos {
            // `Mth.floor(double)` (`Mth.java:65-67`).
            x: (self.x + f64::from(offset.x)).floor() as i32,
            y: (self.y + f64::from(offset.y)).floor() as i32,
            z: (self.z + f64::from(offset.z)).floor() as i32,
        }
    }
}

/// Damage-source values and their actual generation tag authority.
#[derive(Clone, Copy)]
pub struct LootDamageSourceFacts<'a> {
    pub damage_type: &'a str,
    pub is_direct: Option<bool>,
    /// `None` is unavailable authority; `Some(None)` is a known empty registry.
    pub tags: Option<Option<&'a bbb_server_datapack::tag_loader::LoadedRegistryTags>>,
    pub causing: LootEntity<'a>,
    /// `Some(Absent)` is a known null slot; `None` was not retained.
    pub direct: Option<LootEntity<'a>>,
}

#[derive(Clone, Copy, Default)]
pub enum LootDamageSource<'a> {
    #[default]
    Absent,
    Present(LootDamageSourceFacts<'a>),
}

impl LootDamageSource<'_> {
    #[must_use]
    pub fn present(&self) -> bool {
        !matches!(self, Self::Absent)
    }
}

impl std::fmt::Debug for LootDamageSource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Absent => "Absent",
            Self::Present(_) => "Present",
        })
    }
}

/// Which of the six entity keys carry an entity, and what can be read of it.
#[derive(Debug, Clone, Copy, Default)]
pub struct LootEntities<'a> {
    pub this_entity: LootEntity<'a>,
    pub interacting_entity: LootEntity<'a>,
    pub target_entity: LootEntity<'a>,
    pub last_damage_player: LootEntity<'a>,
    pub attacking_entity: LootEntity<'a>,
    pub direct_attacking_entity: LootEntity<'a>,
}

impl<'a> LootEntities<'a> {
    /// What the key named by one `LootContext.EntityTarget`
    /// (`LootContext.java:146-152`) carries. A key that is not one of the six
    /// never carries an entity.
    #[must_use]
    pub fn entity(&self, param: LootContextParam) -> LootEntity<'a> {
        match param {
            Param::ThisEntity => self.this_entity,
            Param::InteractingEntity => self.interacting_entity,
            Param::TargetEntity => self.target_entity,
            Param::LastDamagePlayer => self.last_damage_player,
            Param::AttackingEntity => self.attacking_entity,
            Param::DirectAttackingEntity => self.direct_attacking_entity,
            _ => LootEntity::Absent,
        }
    }

    /// Whether that key carries one at all.
    #[must_use]
    pub fn present(&self, param: LootContextParam) -> bool {
        self.entity(param).present()
    }
}

/// `LootContextParams.BLOCK_ENTITY`: the live object, and the item-stack
/// publication its container components are written through when a node
/// collects them.
#[derive(Clone, Copy)]
pub struct LootBlockEntity<'a> {
    pub block_entity: &'a BlockEntity,
    pub projection: BlockEntityItemProjection<'a>,
}

impl LootBlockEntity<'_> {
    /// `BlockEntity.collectComponents` (`BlockEntity.java:309-314`).
    #[must_use]
    pub fn collect_components(&self) -> CollectedComponents {
        self.block_entity.collect_components(self.projection)
    }
}

impl std::fmt::Debug for LootBlockEntity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LootBlockEntity")
            .field("type_id", &self.block_entity.type_id())
            .field("pos", &self.block_entity.block_pos())
            .finish()
    }
}

/// `ServerLevel.isRaining()` and `isThundering()`, the level reads
/// `WeatherCheck.test` makes (`WeatherCheck.java:24-29`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LootWeather {
    pub raining: bool,
    pub thundering: bool,
}

pub trait LootTextResolver {
    fn resolve(
        &self,
        target: LootContextParam,
        value: &bbb_protocol::NbtSummaryValue,
    ) -> Result<bbb_protocol::NbtSummaryValue, LootRuntimeReason>;
}

impl<F> LootTextResolver for F
where
    F: Fn(
        LootContextParam,
        &bbb_protocol::NbtSummaryValue,
    ) -> Result<bbb_protocol::NbtSummaryValue, LootRuntimeReason>,
{
    fn resolve(
        &self,
        target: LootContextParam,
        value: &bbb_protocol::NbtSummaryValue,
    ) -> Result<bbb_protocol::NbtSummaryValue, LootRuntimeReason> {
        self(target, value)
    }
}

pub type LootVehicleSlots<'a> = dyn Fn(
        &crate::entity::Entity,
        &[i32],
    ) -> Result<Vec<bbb_server_types::item_stack::RuntimeItemStack>, LootRuntimeReason>
    + 'a;

/// The values a caller offers, before a parameter set binds them.
///
/// This is `LootParams.Builder` (`LootParams.java:46-96`) restricted to what
/// this crate can hold: a key with a readable value carries it, and a key whose
/// value only node types outside [`super`]'s declared scope read carries
/// presence alone. [`LootParams::create`] is `LootParams.Builder.create`
/// (`LootParams.java:92-95`).
pub struct LootParams<'a> {
    pub item_environment: Option<&'a dyn super::LootItemEnvironment>,
    pub text_resolver: Option<&'a dyn LootTextResolver>,
    pub vehicle_slots: Option<&'a LootVehicleSlots<'a>>,
    /// The `ContextKeySet` the values are bound to
    /// (`LootParams.java:92-95`).
    pub param_set: &'static LootContextParamSet,
    /// `LootContextParams.ORIGIN`.
    pub origin: Option<LootOrigin>,
    /// `LootContextParams.BLOCK_STATE`, as a global block-state id.
    pub block_state_id: Option<i32>,
    /// `LootContextParams.BLOCK_ENTITY`.
    pub block_entity: Option<LootBlockEntity<'a>>,
    /// `LootParams.dynamicDrops` (`LootParams.java:35-40`): the object whose
    /// `Block.getDrops` override registered its `withDynamicDrop`s, answered by
    /// [`BlockEntity::dynamic_drop`]. `None` registers nothing.
    pub dynamic_drops: Option<&'a BlockEntity>,
    /// `LootContextParams.TOOL`.
    pub tool: Option<&'a LootTool>,
    /// `LootContextParams.EXPLOSION_RADIUS`.
    pub explosion_radius: Option<f32>,
    /// The six entity keys.
    pub entities: LootEntities<'a>,
    /// The actual damage-source values.
    pub damage_source: LootDamageSource<'a>,
    /// `LootContextParams.ENCHANTMENT_LEVEL`, read only by
    /// `EnchantmentLevelProvider` (`EnchantmentLevelProvider.java:20-22`).
    pub enchantment_level: Option<i32>,
    /// `LootContextParams.ENCHANTMENT_ACTIVE`, read only by
    /// `EnchantmentActiveCheck` (`EnchantmentActiveCheck.java:16-18`), likewise.
    pub enchantment_active: Option<bool>,
    /// `LootContextParams.ADDITIONAL_COST_COMPONENT_ALLOWED`, a `Unit` whose
    /// presence is its whole value.
    pub additional_cost_component_allowed: bool,
    /// The actual level resource key; absent facts fail dimension predicates.
    pub dimension: Option<&'a str>,
    /// The loaded blocks a `location_check` reads.
    pub level: Option<&'a LootBlockStateAt<'a>>,
    /// Block metadata and components read by a location predicate.
    pub block_entity_data: Option<&'a super::LootBlockEntityDataAt<'a>>,
    /// The biome a `biomes` location term reads; `None` refuses the term.
    pub biome: Option<&'a super::LootBiomeAt<'a>>,
    /// Current max local raw brightness; `None` refuses a light term.
    pub light: Option<&'a super::LootLightAt<'a>>,
    /// Structure piece membership; `None` refuses a structures term.
    pub structures: Option<&'a super::LootStructuresAt<'a>>,
    /// The sky a `can_see_sky` location term reads; `None` refuses the term.
    pub sky: Option<&'a super::LootSkyAt<'a>>,
    /// The level's weather; `None` refuses a `weather_check`.
    pub weather: Option<LootWeather>,
    /// `LootParams.getLuck` (`LootParams.java:42-44`), the value
    /// `LootPool.addRandomItems` scales `bonus_rolls` by and
    /// `LootPoolSingletonContainer.EntryBase.getWeight` scales `quality` by
    /// (`LootPool.java:96-105`, `LootPoolSingletonContainer.java:121-126`).
    pub luck: f32,
}

impl<'a> LootParams<'a> {
    /// An empty builder for one parameter set: `new LootParams.Builder(level)`
    /// before any `withParameter` (`LootParams.java:46-58`).
    #[must_use]
    pub fn new(param_set: &'static LootContextParamSet) -> Self {
        Self {
            item_environment: None,
            text_resolver: None,
            vehicle_slots: None,
            param_set,
            origin: None,
            block_state_id: None,
            block_entity: None,
            dynamic_drops: None,
            tool: None,
            explosion_radius: None,
            entities: LootEntities::default(),
            damage_source: LootDamageSource::Absent,
            enchantment_level: None,
            enchantment_active: None,
            additional_cost_component_allowed: false,
            dimension: None,
            level: None,
            block_entity_data: None,
            biome: None,
            sky: None,
            light: None,
            structures: None,
            weather: None,
            luck: 0.0,
        }
    }

    /// `ContextMap.has` (`ContextMap.java:18-20`) for the offered values.
    #[must_use]
    pub fn has(&self, param: LootContextParam) -> bool {
        match param {
            Param::Origin => self.origin.is_some(),
            Param::BlockState => self.block_state_id.is_some(),
            Param::BlockEntity => self.block_entity.is_some(),
            Param::Tool => self.tool.is_some(),
            Param::ExplosionRadius => self.explosion_radius.is_some(),
            Param::DamageSource => self.damage_source.present(),
            Param::EnchantmentLevel => self.enchantment_level.is_some(),
            Param::EnchantmentActive => self.enchantment_active.is_some(),
            Param::AdditionalCostComponentAllowed => self.additional_cost_component_allowed,
            entity => self.entities.present(entity),
        }
    }

    /// `LootParams.Builder.create` (`LootParams.java:92-95`) over
    /// `ContextMap.Builder.create` (`ContextMap.java:71-83`): a key outside the
    /// set is refused first, then a required key that was not offered.
    ///
    /// Resolving `BLOCK_STATE` from its id happens here so a whole evaluation
    /// has one place that can answer "no such block state", the way vanilla's
    /// `BlockState` object is resolved before it ever reaches a `LootContext`.
    pub fn create(&self) -> Result<LootContext<'a>, LootRuntimeReason> {
        for param in LOOT_CONTEXT_PARAMS.iter().copied() {
            if self.has(param) && !self.param_set.allows(param) {
                return Err(LootRuntimeReason::ParameterNotAllowed(param));
            }
        }
        for param in self.param_set.required.iter().copied() {
            if !self.has(param) {
                return Err(LootRuntimeReason::MissingRequiredParameter(param));
            }
        }
        let content_registry = self.item_environment.map_or_else(
            || bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
            |env| env.item_resolver().content_registry(),
        );
        let block_state = match self.block_state_id {
            None => None,
            Some(id) => Some(if let Some(identity) = vanilla_block_state_identity(id) {
                std::borrow::Cow::Borrowed(identity)
            } else if let Some((custom, offset)) = content_registry.block_by_state_id(id) {
                let mut properties = std::collections::BTreeMap::new();
                if custom.state_count > 1 || offset > 0 {
                    properties.insert(
                        bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY.to_owned(),
                        offset.to_string(),
                    );
                }
                std::borrow::Cow::Owned(VanillaBlockStateIdentity {
                    id,
                    name: custom.resource_id().to_owned(),
                    properties,
                })
            } else {
                return Err(LootRuntimeReason::UnknownBlockState(id));
            }),
        };
        Ok(LootContext {
            item_environment: self.item_environment,
            text_resolver: self.text_resolver,
            vehicle_slots: self.vehicle_slots,
            additional_cost_component_allowed: self.additional_cost_component_allowed,
            param_set: self.param_set,
            origin: self.origin,
            block_state,
            block_entity: self.block_entity,
            dynamic_drops: self.dynamic_drops,
            tool: self.tool,
            explosion_radius: self.explosion_radius,
            enchantment_level: self.enchantment_level,
            enchantment_active: self.enchantment_active,
            entities: self.entities,
            damage_source: self.damage_source,
            dimension: self.dimension,
            level: self.level,
            block_entity_data: self.block_entity_data,
            biome: self.biome,
            sky: self.sky,
            light: self.light,
            structures: self.structures,
            weather: self.weather,
            luck: self.luck,
        })
    }
}

/// A bound parameter bundle: `LootContext`'s read surface
/// (`LootContext.java:35-45`) for the parameters [`super`] can read.
///
pub struct LootContext<'a> {
    item_environment: Option<&'a dyn super::LootItemEnvironment>,
    text_resolver: Option<&'a dyn LootTextResolver>,
    vehicle_slots: Option<&'a LootVehicleSlots<'a>>,
    additional_cost_component_allowed: bool,
    param_set: &'static LootContextParamSet,
    origin: Option<LootOrigin>,
    block_state: Option<std::borrow::Cow<'static, VanillaBlockStateIdentity>>,
    block_entity: Option<LootBlockEntity<'a>>,
    dynamic_drops: Option<&'a BlockEntity>,
    tool: Option<&'a LootTool>,
    explosion_radius: Option<f32>,
    enchantment_level: Option<i32>,
    enchantment_active: Option<bool>,
    entities: LootEntities<'a>,
    damage_source: LootDamageSource<'a>,
    dimension: Option<&'a str>,
    level: Option<&'a LootBlockStateAt<'a>>,
    block_entity_data: Option<&'a super::LootBlockEntityDataAt<'a>>,
    biome: Option<&'a super::LootBiomeAt<'a>>,
    light: Option<&'a super::LootLightAt<'a>>,
    structures: Option<&'a super::LootStructuresAt<'a>>,
    sky: Option<&'a super::LootSkyAt<'a>>,
    weather: Option<LootWeather>,
    luck: f32,
}

impl std::fmt::Debug for LootContext<'_> {
    /// The level is a trait object with no `Debug` of its own, so it shows as
    /// its presence.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LootContext")
            .field("param_set", &self.param_set.resource_id)
            .field("origin", &self.origin)
            .field(
                "block_state",
                &self.block_state.as_deref().map(|state| state.name.as_str()),
            )
            .field("block_entity", &self.block_entity)
            .field("tool", &self.tool)
            .field("explosion_radius", &self.explosion_radius)
            .field("entities", &self.entities)
            .field("damage_source", &self.damage_source)
            .field("level", &self.level.is_some())
            .field("luck", &self.luck)
            .finish()
    }
}

impl<'a> LootContext<'a> {
    pub(super) fn entity_bodies(&self) -> Option<crate::entity::store::EntityBodyReadView<'a>> {
        [
            self.entities.this_entity,
            self.entities.interacting_entity,
            self.entities.target_entity,
            self.entities.last_damage_player,
            self.entities.attacking_entity,
            self.entities.direct_attacking_entity,
        ]
        .into_iter()
        .find_map(|entity| match entity {
            LootEntity::Stored(stored) => stored.bodies,
            _ => None,
        })
    }

    /// `SetNameFunction.createResolver` (`SetNameFunction.java:69-88`): a
    /// failed resolution, or a path with no command source to resolve
    /// against, keeps the line as written.
    pub(crate) fn resolve_text(
        &self,
        target: LootContextParam,
        value: &bbb_protocol::NbtSummaryValue,
    ) -> bbb_protocol::NbtSummaryValue {
        if !self.entity(target).present() {
            return value.clone();
        }
        self.text_resolver
            .and_then(|resolver| resolver.resolve(target, value).ok())
            .unwrap_or_else(|| value.clone())
    }

    pub(crate) fn item_environment(&self) -> Option<&'a dyn super::LootItemEnvironment> {
        self.item_environment
    }

    pub(super) fn vehicle_slots(&self) -> Option<&'a LootVehicleSlots<'a>> {
        self.vehicle_slots
    }

    /// `LootContextParams.ADDITIONAL_COST_COMPONENT_ALLOWED` as
    /// `EnchantWithLevelsFunction.run` reads it — a bare `hasParameter`
    /// (`EnchantWithLevelsFunction.java:74-86`), false when the set lacks it.
    pub(crate) fn additional_cost_component_allowed(&self) -> bool {
        self.additional_cost_component_allowed
    }

    /// `LootContextParams.TOOL`, read by `MatchTool`
    /// (`MatchTool.java:28-31`), `BonusLevelTableCondition`
    /// (`BonusLevelTableCondition.java:37-42`) and `ApplyBonusCount`
    /// (`ApplyBonusCount.java:63-72`).
    pub(crate) fn tool(&self) -> Option<&'a LootTool> {
        self.tool
    }

    /// `LootContextParams.ORIGIN`, read by `LocationCheck`
    /// (`LocationCheck.java:35-42`).
    pub(crate) fn origin(&self) -> Option<LootOrigin> {
        self.origin
    }

    /// `LootContextParams.BLOCK_STATE`, read by
    /// `LootItemBlockStatePropertyCondition`
    /// (`LootItemBlockStatePropertyCondition.java:44-47`) and `CopyBlockState`
    /// (`CopyBlockState.java:59-74`).
    pub(crate) fn block_state(&self) -> Option<&VanillaBlockStateIdentity> {
        self.block_state.as_deref()
    }

    /// `LootContextParams.BLOCK_ENTITY`, which `CopyComponentsFunction` reads
    /// through `LootContextArg.SimpleGetter.get` (`LootContextArg.java:87-95`).
    pub(crate) fn block_entity(&self) -> Option<LootBlockEntity<'a>> {
        self.block_entity
    }

    /// `LootContext.addDynamicDrops` (`LootContext.java:47-49`)'s registrations.
    pub(crate) fn dynamic_drops(&self) -> Option<&'a BlockEntity> {
        self.dynamic_drops
    }

    /// `LootContextParams.EXPLOSION_RADIUS` for `ExplosionCondition`
    /// (`ExplosionCondition.java:27-36`) and `ApplyExplosionDecay`
    /// (`ApplyExplosionDecay.java:25-43`) alike: only the first names the key,
    /// and naming it moves no read, only the load-time report.
    pub(crate) fn explosion_radius(&self) -> Option<f32> {
        self.explosion_radius
    }

    pub(crate) fn enchantment_level(&self) -> Option<i32> {
        self.enchantment_level
    }

    pub(crate) fn enchantment_active(&self) -> Option<bool> {
        self.enchantment_active
    }

    /// One of the six entity keys, read by whichever node names it through a
    /// `LootContext.EntityTarget` (`LootContext.java:146-152`).
    pub(crate) fn entity(&self, param: LootContextParam) -> LootEntity<'a> {
        self.entities.entity(param)
    }

    /// `LootContextParams.DAMAGE_SOURCE`, read by `DamageSourceCondition.test`
    /// (`DamageSourceCondition.java:29-33`).
    pub(crate) fn damage_source(&self) -> LootDamageSource<'a> {
        self.damage_source
    }

    pub(crate) fn dimension(&self) -> Option<&'a str> {
        self.dimension
    }

    /// `LootContext.getLevel` (`LootContext.java:75-77`) as far as this crate
    /// reads it.
    pub(crate) fn level(&self) -> Option<&'a LootBlockStateAt<'a>> {
        self.level
    }

    pub(crate) fn block_entity_data(&self) -> Option<&'a super::LootBlockEntityDataAt<'a>> {
        self.block_entity_data
    }

    pub(crate) fn biome(&self) -> Option<&'a super::LootBiomeAt<'a>> {
        self.biome
    }

    pub(crate) fn sky(&self) -> Option<&'a super::LootSkyAt<'a>> {
        self.sky
    }

    pub(crate) fn light(&self) -> Option<&'a super::LootLightAt<'a>> {
        self.light
    }

    pub(crate) fn structures(&self) -> Option<&'a super::LootStructuresAt<'a>> {
        self.structures
    }

    /// `LootContext.getLuck` (`LootContext.java:71-73`).
    pub(crate) fn luck(&self) -> f32 {
        self.luck
    }

    pub(crate) fn weather(&self) -> Option<LootWeather> {
        self.weather
    }
}
