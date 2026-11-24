//! Canonical server-side damage dispatch for stored non-player entities.
//!
//! [`bbb_server_datapack::damage`] owns the much larger player pipeline. This module owns
//! the closed [`EntityData`] set held by an [`EntityStore`]: item entities,
//! experience orbs and falling blocks. Keeping the dispatch here makes a new
//! live entity variant choose its damage semantics at compile time instead of
//! silently inheriting a generic approximation.

use crate::world::simulation_level::LevelCtx;
mod environment;
pub(crate) use environment::{
    base_tick_from_environment, hurt_detached_from_environment, supports_detached_environment_hurt,
};

use bbb_protocol::entity_types::vanilla_entity_type_is_fire_immune;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::ServerWorldError;

use crate::entity::item::spawn_item_entity;
use crate::entity::{Entity, EntityData, EntityStore};
use crate::explosion::{ExplosionSourceEntity, ExplosionSourceEntityKind};
use crate::level_event::{LevelEventSink, VanillaGameEvent};
use bbb_server_datapack::damage::DamageTypeFacts;
use bbb_server_types::geometry::Vec3Ops;

/// The causing entity returned by `DamageSource.getEntity()`.
///
/// The base invulnerability check distinguishes a creative player, while
/// `ItemEntity.hurtServer` distinguishes any mob when `mobGriefing` is false.
/// Other target-independent properties of living entities do not enter any
/// stored subtype's damage path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoredDamageCausingEntity {
    Player {
        uuid: uuid::Uuid,
        entity_id: i32,
        instabuild: bool,
    },
    Mob {
        entity_id: i32,
    },
    Other {
        entity_id: i32,
    },
}

impl StoredDamageCausingEntity {
    /// `DamageSource.getEntity().getId()`, used as the game-event context.
    #[must_use]
    pub const fn entity_id(self) -> i32 {
        match self {
            Self::Player { entity_id, .. }
            | Self::Mob { entity_id }
            | Self::Other { entity_id } => entity_id,
        }
    }

    /// The UUID `Explosion.getIndirectSourceEntity` uses for player-caused
    /// explosions.
    #[must_use]
    pub const fn player_uuid(self) -> Option<uuid::Uuid> {
        match self {
            Self::Player { uuid, .. } => Some(uuid),
            Self::Mob { .. } | Self::Other { .. } => None,
        }
    }

    #[must_use]
    pub fn uuid(self, store: &EntityStore) -> Option<uuid::Uuid> {
        match self {
            Self::Player { uuid, .. } => Some(uuid),
            Self::Mob { entity_id } | Self::Other { entity_id } => {
                store.get(entity_id).map(Entity::uuid)
            }
        }
    }

    pub(crate) const fn is_creative_player(self) -> bool {
        matches!(
            self,
            Self::Player {
                instabuild: true,
                ..
            }
        )
    }

    pub const fn is_mob(self) -> bool {
        matches!(self, Self::Mob { .. })
    }
}

/// What `LivingEntity.hurtServer` reads off `source.getEntity()` to blame
/// the hurt (`LivingEntity.java:1329-1351`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DamageCauser {
    pub(crate) uuid: Option<uuid::Uuid>,
    pub(crate) living: bool,
    /// A tame wolf's owner; `Some(None)` for a tame wolf without one.
    pub(crate) tame_wolf_owner: Option<Option<uuid::Uuid>>,
    /// `Breeze.isInvulnerableTo` (`Breeze.java:263-264`) refuses this source.
    pub(crate) breeze: bool,
}

impl DamageCauser {
    /// `body` answers a stored causing entity by id.
    pub(crate) fn resolve<'e>(
        causing: Option<StoredDamageCausingEntity>,
        body: impl FnOnce(i32) -> Option<&'e Entity>,
    ) -> Self {
        match causing {
            None => Self::default(),
            Some(StoredDamageCausingEntity::Player { uuid, .. }) => Self {
                uuid: Some(uuid),
                living: true,
                tame_wolf_owner: None,
                breeze: false,
            },
            Some(causing) => body(causing.entity_id()).map_or_else(Self::default, |entity| Self {
                uuid: Some(entity.uuid()),
                living: entity.living().is_some(),
                tame_wolf_owner: tame_wolf_owner(entity),
                breeze: entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID,
            }),
        }
    }
}

/// A tame wolf's owner; `Some(None)` for a tame wolf without one.
pub(crate) fn tame_wolf_owner(entity: &Entity) -> Option<Option<uuid::Uuid>> {
    match entity.data() {
        EntityData::GeneralLiving(data)
            if entity.entity_type_id()
                == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WOLF_ID =>
        {
            data.tamable_retained()
                .filter(|wolf| wolf.is_tame())
                .map(|wolf| wolf.owner())
        }
        _ => None,
    }
}

pub fn explosion_source_entity(entity: &Entity) -> ExplosionSourceEntity {
    let kind = if matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_mob()) {
        ExplosionSourceEntityKind::Mob
    } else if entity.living().is_some() {
        ExplosionSourceEntityKind::LivingOther
    } else {
        ExplosionSourceEntityKind::Other
    };
    ExplosionSourceEntity {
        custom_name: entity.custom_name().cloned(),
        entity_id: entity.id(),
        entity_resource_id: entity.data().entity_resource_id().to_string(),
        uuid: entity.uuid(),
        position: entity.position(),
        kind,
    }
}

pub fn living_explosion_source_entity(entity: &Entity) -> Option<ExplosionSourceEntity> {
    entity.living()?;
    Some(explosion_source_entity(entity))
}

pub(crate) fn explosion_owner_source_entity(
    owner: uuid::Uuid,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<ExplosionSourceEntity> {
    if let Some(player) = level.player(owner) {
        return Some(ExplosionSourceEntity {
            custom_name: None,
            entity_id: player.entity_id,
            entity_resource_id: "minecraft:player".to_string(),
            uuid: player.uuid,
            position: player.position,
            kind: ExplosionSourceEntityKind::Player {
                instabuild: player.instabuild,
                may_build: level.player_may_build(owner),
                spectator: player.spectator,
            },
        });
    }
    let entity_id = store.entity_id_for_uuid(owner)?;
    living_explosion_source_entity(store.get(entity_id)?)
}

pub fn damage_causing_explosion_source_entity(
    causing: StoredDamageCausingEntity,
    store: &EntityStore,
    player_may_build: bool,
    player_spectator: Option<bool>,
    player_position: Option<bbb_protocol::packets::Vec3d>,
) -> Option<ExplosionSourceEntity> {
    match causing {
        StoredDamageCausingEntity::Player {
            uuid,
            entity_id,
            instabuild,
        } => Some(ExplosionSourceEntity {
            custom_name: None,
            entity_id,
            entity_resource_id: "minecraft:player".to_string(),
            uuid,
            position: player_position?,
            kind: ExplosionSourceEntityKind::Player {
                instabuild,
                may_build: player_may_build,
                spectator: player_spectator?,
            },
        }),
        StoredDamageCausingEntity::Mob { entity_id }
        | StoredDamageCausingEntity::Other { entity_id } => {
            store.get(entity_id).map(explosion_source_entity)
        }
    }
}

pub(crate) fn tnt_minecart_damage_source(
    source: StoredEntityDamageSource<'_>,
    causing_entity: Option<ExplosionSourceEntity>,
    store: &EntityStore,
) -> (crate::entity::TntMinecartDamageSource, Option<f64>) {
    let direct = source.direct_entity_id.and_then(|id| store.get(id));
    let direct_entity_is_projectile = direct.is_some_and(Entity::is_projectile_class);
    let direct_projectile_is_on_fire = direct_entity_is_projectile
        && direct.is_some_and(|entity| entity.remaining_fire_ticks() > 0);
    let burning_arrow_speed_squared = direct
        .filter(|entity| {
            direct_projectile_is_on_fire
                && matches!(
                    entity.data(),
                    EntityData::Arrow(_) | EntityData::SpectralArrow(_) | EntityData::Trident(_)
                )
        })
        .map(|entity| entity.delta_movement().length_sqr());
    (
        crate::entity::TntMinecartDamageSource {
            ignition_source: crate::entity::TntMinecartIgnitionSource::new(causing_entity),
            direct_entity_is_projectile,
            direct_projectile_is_on_fire,
            is_fire: source.damage_type.is_fire,
            is_explosion: source.damage_type.is_explosion,
        },
        burning_arrow_speed_squared,
    )
}

/// The target-independent half of one `DamageSource`.
#[derive(Clone, Copy)]
pub struct StoredEntityDamageSource<'a> {
    pub damage_type: &'a DamageTypeFacts,
    /// `DamageSource.getDirectEntity()`. No current stored subtype reads this,
    /// but the production seam retains it rather than collapsing a projectile
    /// source into its owner.
    pub direct_entity_id: Option<i32>,
    /// `DamageSource.getEntity()`, the causing owner used by the rules and by
    /// `GameEvent.Context.sourceEntity`.
    pub causing_entity: Option<StoredDamageCausingEntity>,
    /// The two entities above as loot context views, for the one of them this
    /// crate holds no state for.
    pub attacker_views: StoredAttackerViews<'a>,
}

impl<'a> StoredEntityDamageSource<'a> {
    /// The source for a damage path the level has no views for:
    /// `LootContextParams.ATTACKING_ENTITY` and `DIRECT_ATTACKING_ENTITY` then
    /// carry the ids alone, and a table reading either slot is refused rather
    /// than answered about the wrong entity.
    pub fn without_attacker_views(
        damage_type: &'a DamageTypeFacts,
        direct_entity_id: Option<i32>,
        causing_entity: Option<StoredDamageCausingEntity>,
    ) -> Self {
        Self {
            damage_type,
            direct_entity_id,
            causing_entity,
            attacker_views: StoredAttackerViews::default(),
        }
    }
}

impl std::fmt::Debug for StoredEntityDamageSource<'_> {
    /// The views have no `Debug` of their own, so each shows as its presence.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredEntityDamageSource")
            .field("damage_type", &self.damage_type.resource_id)
            .field("direct_entity_id", &self.direct_entity_id)
            .field("causing_entity", &self.causing_entity)
            .field("causing_view", &self.attacker_views.causing.is_some())
            .field("direct_view", &self.attacker_views.direct.is_some())
            .finish()
    }
}

/// `DamageSource`'s two entities as loot snapshots.
///
#[derive(Clone, Copy, Default)]
pub struct StoredAttackerViews<'a> {
    /// `source.getEntity()`.
    pub causing: Option<&'a crate::loot_runtime::LootEntityFacts>,
    /// `source.getDirectEntity()`.
    pub direct: Option<&'a crate::loot_runtime::LootEntityFacts>,
}

impl StoredAttackerViews<'_> {
    pub const fn is_empty(&self) -> bool {
        self.causing.is_none() && self.direct.is_none()
    }
}

/// World rules consulted by the stored-entity damage path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredEntityDamageRules {
    pub mob_griefing: bool,
    pub entity_drops: bool,
    pub tnt_explodes: bool,
    /// Membership of the target's type in the world resolver's
    /// `#minecraft:fall_damage_immune` entity-type facts. The entity owner
    /// consumes the projected answer rather than choosing a tag generation.
    pub fall_damage_immune: bool,
}

impl Default for StoredEntityDamageRules {
    fn default() -> Self {
        Self {
            mob_griefing: true,
            entity_drops: true,
            tnt_explodes: true,
            fall_damage_immune: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredEntityDamageContext {
    pub block_drops: bool,
    pub attacker_may_build: bool,
    pub attacker_spectator: Option<bool>,
}

impl Default for StoredEntityDamageContext {
    fn default() -> Self {
        Self {
            block_drops: true,
            attacker_may_build: true,
            attacker_spectator: None,
        }
    }
}

/// `Item.onDestroyed(ItemEntity)`: the cleared source stack and its released
/// non-empty contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemOnDestroyed {
    pub source_stack: RuntimeItemStack,
    pub drops: Vec<RuntimeItemStack>,
}

pub fn item_on_destroyed(
    stack: &RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> bbb_server_types::Result<ItemOnDestroyed> {
    use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID as BUNDLE;
    let unchanged = || ItemOnDestroyed {
        source_stack: stack.clone(),
        drops: Vec::new(),
    };
    let Some(item) = stack
        .item_id()
        .and_then(bbb_protocol::item_types::vanilla_item_type)
    else {
        return Ok(unchanged());
    };
    if item.block_item_behavior.is_some() {
        return Ok(ItemOnDestroyed {
            drops: stack
                .container_contents()
                .unwrap_or_default()
                .into_iter()
                .filter(|item| !item.is_empty())
                .collect(),
            source_stack: stack.with_container_contents(&[], resolver)?,
        });
    }
    if item.use_override != Some(bbb_protocol::item_types::VanillaItemUseOverride::BundleItem) {
        return Ok(unchanged());
    }
    let Some(contents) = stack.effective_component(BUNDLE, resolver)? else {
        return Ok(unchanged());
    };
    Ok(ItemOnDestroyed {
        drops: contents
            .bundle_contents_items
            .iter()
            .map(RuntimeItemStack::from_template)
            .filter(|item| !item.is_empty())
            .collect(),
        source_stack: stack.clone().with_component_nbt(
            BUNDLE,
            Some(&bbb_protocol::NbtSummaryValue::List(Vec::new())),
            resolver,
        )?,
    })
}

pub fn item_resists_damage(
    stack: &RuntimeItemStack,
    damage_type_resource_id: &str,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> bool {
    use bbb_protocol::{NbtSummaryValue, PersistentRegistryTagMembers};
    let Ok(Some(NbtSummaryValue::Compound(fields))) = stack.component_nbt(
        bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID,
        resolver,
    ) else {
        return false;
    };
    let normalize = |id: &str| bbb_protocol::normalize_resource_location(id.to_owned()).ok();
    let damage_type = normalize(damage_type_resource_id);
    let ids = match fields
        .iter()
        .find(|field| field.name == "types")
        .map(|field| &field.value)
    {
        Some(NbtSummaryValue::String(raw)) => match raw.try_to_string() {
            Ok(raw) => match raw.strip_prefix('#') {
                Some(tag) => match resolver.tag_members("minecraft:damage_type", tag) {
                    PersistentRegistryTagMembers::Present(members) => members,
                    _ => Vec::new(),
                },
                None => vec![raw],
            },
            Err(_) => Vec::new(),
        },
        Some(NbtSummaryValue::List(values)) => values
            .iter()
            .filter_map(|value| match value {
                NbtSummaryValue::String(id) => id.try_to_string().ok(),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    ids.iter().any(|id| normalize(id) == damage_type)
}

/// What the target subtype's `hurtServer` did and what it returned.
///
/// `FallingBlockEntity` is the important split: it applies `markHurt` but
/// returns false. Commands must retain that effect while still producing the
/// same invulnerable error Vanilla derives from the return value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredEntityHurtOutcome {
    pub applied: bool,
    pub succeeded: bool,
}

#[derive(Debug)]
pub(crate) struct EntityHurtMutation {
    pub(crate) outcome: StoredEntityHurtOutcome,
    pub(crate) emits_entity_damage: bool,
    pub(crate) emits_entity_die: bool,
    pub(crate) game_event_source_id: Option<i32>,
    pub(crate) destroyed_drops: Vec<RuntimeItemStack>,
    pub(crate) removal_before_drops: bool,
}

/// Evaluates canonical stored-entity damage against a clone of `entity`.
///
/// This is the command-feedback path: it runs the same exhaustive subtype
/// function and item-fact queries as live application, including the
/// falling-block mark-but-false distinction, but publishes no event, child or
/// lifecycle edge and leaves the supplied snapshot unchanged.
pub fn evaluate_stored_entity_damage(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
    items: &dyn PersistentItemStackHolderResolver,
) -> Result<StoredEntityHurtOutcome, ServerWorldError> {
    evaluate_stored_entity_damage_in_context(
        entity,
        source,
        damage,
        rules,
        StoredEntityDamageContext::default(),
        items,
    )
}

/// [`evaluate_stored_entity_damage`] with the armor stand's extra facts.
pub fn evaluate_stored_entity_damage_in_context(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    items: &dyn PersistentItemStackHolderResolver,
) -> Result<StoredEntityHurtOutcome, ServerWorldError> {
    let mut projection = entity.clone();
    if matches!(projection.data(), EntityData::EndCrystal(_)) {
        return Ok(
            crate::entity::end_crystal::hurt_decision(&mut projection, source, rules).outcome,
        );
    }
    hurt_entity(
        &mut projection,
        source,
        damage,
        rules,
        context,
        items,
        DamageCauser::default(),
    )
    .map(|mutation| mutation.outcome)
}

impl EntityStore {
    /// Runs the target subtype's canonical `hurtServer` implementation.
    ///
    /// The entity is detached while subtype logic runs. Vehicle removal is
    /// published before its drop; item children retain their subtype order.
    pub fn hurt_server(
        &mut self,
        entity_id: i32,
        source: StoredEntityDamageSource<'_>,
        damage: f32,
        rules: StoredEntityDamageRules,
        items: &dyn PersistentItemStackHolderResolver,
        event_sink: &mut LevelEventSink,
    ) -> Result<Option<StoredEntityHurtOutcome>, ServerWorldError> {
        self.hurt_server_in_context(
            entity_id,
            source,
            damage,
            rules,
            StoredEntityDamageContext::default(),
            items,
            event_sink,
        )
    }

    /// [`EntityStore::hurt_server`] with the armor stand's extra facts.
    #[allow(clippy::too_many_arguments)]
    pub fn hurt_server_in_context(
        &mut self,
        entity_id: i32,
        source: StoredEntityDamageSource<'_>,
        damage: f32,
        rules: StoredEntityDamageRules,
        context: StoredEntityDamageContext,
        items: &dyn PersistentItemStackHolderResolver,
        event_sink: &mut LevelEventSink,
    ) -> Result<Option<StoredEntityHurtOutcome>, ServerWorldError> {
        self.hurt_server_in_context_with_attacker_position(
            entity_id, source, damage, rules, context, None, items, event_sink,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn hurt_server_in_context_with_attacker_position(
        &mut self,
        entity_id: i32,
        source: StoredEntityDamageSource<'_>,
        damage: f32,
        rules: StoredEntityDamageRules,
        context: StoredEntityDamageContext,
        attacker_position: Option<bbb_protocol::packets::Vec3d>,
        items: &dyn PersistentItemStackHolderResolver,
        event_sink: &mut LevelEventSink,
    ) -> Result<Option<StoredEntityHurtOutcome>, ServerWorldError> {
        let Some(mut entity) = self.detach(entity_id) else {
            return Ok(None);
        };
        if matches!(entity.data(), EntityData::ShulkerBullet(_)) {
            crate::entity::shulker_bullet::hurt_server(&mut entity, self);
            let event_position = entity.position();
            self.attach(entity);
            event_sink.game_event_with_source_at(
                VanillaGameEvent::ENTITY_DAMAGE,
                event_position,
                Some(entity_id),
                None,
            );
            return Ok(Some(StoredEntityHurtOutcome {
                applied: true,
                succeeded: true,
            }));
        }
        if matches!(entity.data(), EntityData::EndCrystal(_)) {
            let causing_entity_snapshot = source.causing_entity.and_then(|causing| {
                damage_causing_explosion_source_entity(
                    causing,
                    self,
                    context.attacker_may_build,
                    context.attacker_spectator,
                    attacker_position,
                )
            });
            let causing_is_ender_dragon = source.causing_entity.is_some_and(|causing| {
                self.get(causing.entity_id()).is_some_and(|causing| {
                    causing.data().entity_resource_id() == "minecraft:ender_dragon"
                })
            });
            let decision = crate::entity::end_crystal::hurt_decision_with_causing_type(
                &mut entity,
                source,
                rules,
                causing_is_ender_dragon,
                causing_entity_snapshot,
            );
            if let Some(explosion) = decision.pending_explosion {
                self.queue_command(crate::entity::EntityCommand::Explosion(explosion));
            }
            self.attach(entity);
            return Ok(Some(decision.outcome));
        }
        if matches!(entity.data(), EntityData::TntMinecart(_)) {
            let causing_entity_snapshot = source.causing_entity.and_then(|causing| {
                damage_causing_explosion_source_entity(
                    causing,
                    self,
                    context.attacker_may_build,
                    context.attacker_spectator,
                    attacker_position,
                )
            });
            let (tnt_source, burning_arrow_speed_squared) =
                tnt_minecart_damage_source(source, causing_entity_snapshot, self);
            let position = entity.position();
            let minecart_source = crate::entity::MinecartHurtSource {
                creative_player: source
                    .causing_entity
                    .is_some_and(StoredDamageCausingEntity::is_creative_player),
                bypasses_invulnerability: source.damage_type.bypasses_invulnerability,
                is_fire: source.damage_type.is_fire,
                fire_immune: entity.plugin_mob().is_some_and(|plugin| plugin.fire_immune)
                    || vanilla_entity_type_is_fire_immune(entity.entity_type_id()),
                is_fall: source.damage_type.is_fall,
                fall_damage_immune: rules.fall_damage_immune,
                causing_entity_id: source
                    .causing_entity
                    .map(StoredDamageCausingEntity::entity_id),
            };
            let effects = crate::entity::minecart::hurt_tnt_server(
                &mut entity,
                self,
                damage,
                minecart_source,
                tnt_source,
                burning_arrow_speed_squared,
                rules.tnt_explodes,
                rules.entity_drops,
            );
            if effects.emits_entity_damage {
                event_sink.game_event_with_source_at(
                    VanillaGameEvent::ENTITY_DAMAGE,
                    entity.position(),
                    effects.game_event_source_id,
                    None,
                );
            }
            let outcome = StoredEntityHurtOutcome {
                applied: effects.applied,
                succeeded: effects.succeeded,
            };
            let drops = effects
                .drops
                .into_iter()
                .map(|stack| {
                    crate::entity::named_entity_drop::name_vehicle_drop(
                        &entity,
                        stack,
                        self.item_resolver(),
                    )
                })
                .collect();
            let attached = self.attach_checked(entity);
            if attached.is_none() || spawn_destroyed_drops(self, position, drops).is_none() {
                return Ok(None);
            }
            return Ok(Some(outcome));
        }
        if matches!(entity.data(), EntityData::ArmorStand(_)) {
            let Some(effects) = crate::entity::armor_stand::hurt_server(
                &mut entity,
                self,
                source,
                damage,
                rules,
                context,
            ) else {
                self.attach(entity);
                return Ok(None);
            };
            if effects.emits_entity_damage {
                event_sink.game_event_with_source_at(
                    VanillaGameEvent::ENTITY_DAMAGE,
                    entity.position(),
                    source
                        .causing_entity
                        .map(StoredDamageCausingEntity::entity_id),
                    None,
                );
            }
            let died_as = effects
                .emits_entity_die
                .then(|| (entity.position(), entity.id()));
            self.attach(entity);
            if let Some((position, id)) = died_as {
                crate::entity::kill::raise_entity_die_in_sink(event_sink, position, id);
            }
            return Ok(Some(effects.outcome));
        }
        let position = entity.position();
        if matches!(
            entity.data(),
            crate::entity::EntityData::Painting(_)
                | crate::entity::EntityData::ItemFrame(_)
                | crate::entity::EntityData::GlowItemFrame(_)
                | crate::entity::EntityData::LeashKnot(_)
        ) {
            let Some(effects) = crate::entity::hanging::hurt_server_with_post_actions(
                &mut entity,
                self,
                rules.entity_drops,
                crate::entity::HangHurtSource {
                    is_explosion: source.damage_type.is_explosion,
                    is_fire: source.damage_type.is_fire,
                    is_fall: source.damage_type.is_fall,
                    fall_damage_immune: rules.fall_damage_immune,
                    from_mob: source
                        .causing_entity
                        .is_some_and(StoredDamageCausingEntity::is_mob),
                    mob_griefing: rules.mob_griefing,
                    caused_by_entity: source.causing_entity.is_some(),
                    creative_player: source
                        .causing_entity
                        .is_some_and(StoredDamageCausingEntity::is_creative_player),
                    bypasses_invulnerability: source.damage_type.bypasses_invulnerability,
                },
            ) else {
                self.attach(entity);
                return Ok(None);
            };
            let outcome = effects.outcome;
            let died_as = effects
                .emits_entity_die
                .then(|| (entity.position(), entity.id()));
            self.attach(entity);
            if let Some((position, id)) = died_as {
                crate::entity::kill::raise_entity_die_in_sink(event_sink, position, id);
            }
            crate::entity::hanging::apply_hanging_post_actions(
                effects.actions,
                self,
                event_sink,
                source
                    .causing_entity
                    .map(StoredDamageCausingEntity::entity_id),
            );
            return Ok(Some(StoredEntityHurtOutcome {
                applied: outcome,
                succeeded: outcome,
            }));
        }
        // The victim is detached by now, so a source that names the victim
        // itself answers no UUID here and the reference carries the id alone,
        // which this store answers again once the tick files the entity back.
        let causer = DamageCauser::resolve(source.causing_entity, |id| self.get(id));
        let mutation = match hurt_entity(&mut entity, source, damage, rules, context, items, causer)
        {
            Ok(mutation) => mutation,
            Err(error) => {
                self.attach(entity);
                return Err(error);
            }
        };
        crate::entity::living::mob_effects::on_mob_hurt(&entity, self, mutation.outcome.succeeded);
        if mutation.outcome.succeeded {
            crate::entity::general_living::abstract_horse_hurt_follow_up(&mut entity, self);
        }
        queue_container_vehicle_anger(self, &entity, mutation.emits_entity_die, source, rules);

        if mutation.emits_entity_damage {
            event_sink.game_event_with_source_at(
                VanillaGameEvent::ENTITY_DAMAGE,
                entity.position(),
                mutation.game_event_source_id,
                None,
            );
        }
        let died_as = mutation
            .emits_entity_die
            .then(|| (entity.position(), entity.id()));
        let drops = mutation.destroyed_drops;
        if mutation.removal_before_drops {
            if self.attach_checked(entity).is_none() {
                return Ok(None);
            }
            if let Some((died_position, id)) = died_as {
                crate::entity::kill::raise_entity_die_in_sink(event_sink, died_position, id);
            }
            if spawn_destroyed_drops(self, position, drops).is_none() {
                return Ok(None);
            }
        } else {
            if let Some((died_position, id)) = died_as {
                crate::entity::kill::raise_entity_die_in_sink(event_sink, died_position, id);
            }
            let published = spawn_destroyed_drops(self, position, drops);
            if self.attach_checked(entity).is_none() {
                return Ok(None);
            }
            if published.is_none() {
                return Ok(None);
            }
        }
        Ok(Some(mutation.outcome))
    }
}

pub(crate) fn spawn_destroyed_drops(
    store: &mut EntityStore,
    position: bbb_protocol::packets::Vec3d,
    drops: Vec<RuntimeItemStack>,
) -> Option<()> {
    for stack in drops.into_iter().filter(|stack| !stack.is_empty()) {
        spawn_item_entity(store, position, stack)?;
    }
    Some(())
}

/// `ContainerEntity.chestVehicleDestroyed` (`ContainerEntity.java:79-86`):
/// a player's own blow that destroys a container vehicle angers piglins.
pub(crate) fn queue_container_vehicle_anger(
    store: &mut EntityStore,
    vehicle: &Entity,
    destroyed: bool,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
) {
    if !destroyed
        || !rules.entity_drops
        || crate::entity::chest_boat::chest_boat(vehicle.data()).is_none()
            && crate::entity::minecart_container::minecart_container(vehicle.data()).is_none()
    {
        return;
    }
    if let Some(StoredDamageCausingEntity::Player {
        uuid, entity_id, ..
    }) = source.causing_entity
    {
        if source.direct_entity_id == Some(entity_id) {
            store.queue_command(crate::entity::EntityCommand::AngerNearbyPiglins { player: uuid });
        }
    }
}

pub(crate) fn hurt_entity(
    entity: &mut Entity,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    items: &dyn PersistentItemStackHolderResolver,
    causer: DamageCauser,
) -> Result<EntityHurtMutation, ServerWorldError> {
    if matches!(entity.data(), EntityData::ShulkerBullet(_)) {
        entity.discard();
        return Ok(EntityHurtMutation {
            outcome: StoredEntityHurtOutcome {
                applied: true,
                succeeded: true,
            },
            emits_entity_damage: false,
            game_event_source_id: None,
            destroyed_drops: Vec::new(),
            emits_entity_die: false,
            removal_before_drops: false,
        });
    }
    if matches!(entity.data(), EntityData::ArmorStand(_)) {
        let decision = crate::entity::armor_stand::hurt_decision(entity, source, rules, context);
        return Ok(EntityHurtMutation {
            outcome: decision.outcome(),
            emits_entity_damage: false,
            game_event_source_id: None,
            destroyed_drops: Vec::new(),
            emits_entity_die: false,
            removal_before_drops: false,
        });
    }
    if matches!(entity.data(), EntityData::GeneralLiving(_)) {
        let effects =
            crate::entity::general_living::hurt_server(entity, source, damage, rules, causer);
        return Ok(EntityHurtMutation {
            outcome: effects.outcome,
            emits_entity_damage: effects.emits_entity_damage,
            game_event_source_id: effects.game_event_source_id,
            destroyed_drops: Vec::new(),
            emits_entity_die: false,
            removal_before_drops: false,
        });
    }
    if crate::entity::boat::is_boat(entity.data()) {
        let effects = crate::entity::boat::hurt_server(
            entity,
            damage,
            crate::entity::BoatHurtSource {
                creative_player: source
                    .causing_entity
                    .is_some_and(StoredDamageCausingEntity::is_creative_player),
                bypasses_invulnerability: source.damage_type.bypasses_invulnerability,
                is_fire: source.damage_type.is_fire,
                fire_immune: entity.plugin_mob().is_some_and(|plugin| plugin.fire_immune)
                    || vanilla_entity_type_is_fire_immune(entity.entity_type_id()),
                is_fall: source.damage_type.is_fall,
                fall_damage_immune: rules.fall_damage_immune,
                causing_entity_id: source
                    .causing_entity
                    .map(StoredDamageCausingEntity::entity_id),
            },
            rules.entity_drops,
        );
        return Ok(EntityHurtMutation {
            outcome: StoredEntityHurtOutcome {
                applied: effects.applied,
                succeeded: effects.succeeded,
            },
            emits_entity_damage: effects.emits_entity_damage,
            emits_entity_die: effects.emits_entity_die,
            game_event_source_id: effects.game_event_source_id,
            destroyed_drops: effects
                .drops
                .into_iter()
                .map(|stack| {
                    crate::entity::named_entity_drop::name_vehicle_drop(entity, stack, items)
                })
                .collect(),
            removal_before_drops: true,
        });
    }
    if crate::entity::minecart::is_minecart(entity.data()) {
        let effects = crate::entity::minecart::hurt_server(
            entity,
            damage,
            crate::entity::MinecartHurtSource {
                creative_player: source
                    .causing_entity
                    .is_some_and(StoredDamageCausingEntity::is_creative_player),
                bypasses_invulnerability: source.damage_type.bypasses_invulnerability,
                is_fire: source.damage_type.is_fire,
                fire_immune: entity.plugin_mob().is_some_and(|plugin| plugin.fire_immune)
                    || vanilla_entity_type_is_fire_immune(entity.entity_type_id()),
                is_fall: source.damage_type.is_fall,
                fall_damage_immune: rules.fall_damage_immune,
                causing_entity_id: source
                    .causing_entity
                    .map(StoredDamageCausingEntity::entity_id),
            },
            rules.entity_drops,
        );
        return Ok(EntityHurtMutation {
            outcome: StoredEntityHurtOutcome {
                applied: effects.applied,
                succeeded: effects.succeeded,
            },
            emits_entity_damage: effects.emits_entity_damage,
            emits_entity_die: effects.emits_entity_die,
            game_event_source_id: effects.game_event_source_id,
            destroyed_drops: effects
                .drops
                .into_iter()
                .map(|stack| {
                    crate::entity::named_entity_drop::name_vehicle_drop(entity, stack, items)
                })
                .collect(),
            removal_before_drops: true,
        });
    }
    if matches!(
        entity.data(),
        EntityData::Marker(_)
            | EntityData::Interaction(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_)
            | EntityData::LargeFireball(_)
            | EntityData::SmallFireball(_)
            | EntityData::DragonFireball(_)
            | EntityData::WitherSkull(_)
            | EntityData::WindCharge(_)
            | EntityData::BreezeWindCharge(_)
            | EntityData::AreaEffectCloud(_)
            | EntityData::OminousItemSpawner(_)
    ) {
        return Ok(refused());
    }
    if removed_or_invulnerable_to_damage(entity, source) {
        return Ok(refused());
    }
    if source.damage_type.is_fire && fire_immune(entity, items)? {
        return Ok(refused());
    }
    if source.damage_type.is_fall && rules.fall_damage_immune {
        return Ok(refused());
    }

    match entity.data() {
        EntityData::Player | EntityData::EnderDragonPart => Ok(refused()),
        EntityData::Item(item) => {
            if !rules.mob_griefing
                && source
                    .causing_entity
                    .is_some_and(StoredDamageCausingEntity::is_mob)
            {
                return Ok(refused());
            }
            if item_resists_damage(item.item(), &source.damage_type.resource_id, items) {
                return Ok(refused());
            }

            let health = java_health_after_damage(item.health(), damage);
            entity.mark_hurt();
            let EntityData::Item(item) = &mut entity.data else {
                unreachable!("the exhaustive dispatch selected an item")
            };
            item.set_health(health);
            let destroyed_drops = if health <= 0 {
                let destroyed = item_on_destroyed(item.item(), items)?;
                item.update_item(destroyed.source_stack);
                entity.discard();
                destroyed.drops
            } else {
                Vec::new()
            };
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: true,
                },
                emits_entity_damage: true,
                game_event_source_id: source
                    .causing_entity
                    .map(StoredDamageCausingEntity::entity_id),
                destroyed_drops,
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::ExperienceOrb(orb) => {
            let health = java_health_after_damage(orb.health(), damage);
            entity.mark_hurt();
            let EntityData::ExperienceOrb(orb) = &mut entity.data else {
                unreachable!("the exhaustive dispatch selected an experience orb")
            };
            orb.set_health(health);
            if health <= 0 {
                entity.discard();
            }
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: true,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::Tnt(_) => Ok(refused()),
        EntityData::EyeOfEnder(_) => Ok(refused()),
        EntityData::FallingBlock(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::Arrow(_)
        | EntityData::SpectralArrow(_)
        | EntityData::SplashPotion(_)
        | EntityData::LingeringPotion(_) => {
            // `Entity.hurtServer` default: projectiles are not damaged in play.
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::Trident(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::FishingBobber(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::Snowball(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::Egg(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::EnderPearl(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::ExperienceBottle(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::FireworkRocket(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::LlamaSpit(_) => {
            entity.mark_hurt();
            Ok(EntityHurtMutation {
                outcome: StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                },
                emits_entity_damage: false,
                game_event_source_id: None,
                destroyed_drops: Vec::new(),
                emits_entity_die: false,
                removal_before_drops: false,
            })
        }
        EntityData::EvokerFangs(_) | EntityData::LightningBolt(_) => Ok(refused()),
        EntityData::Painting(_)
        | EntityData::ItemFrame(_)
        | EntityData::GlowItemFrame(_)
        | EntityData::LeashKnot(_) => {
            unreachable!("hanging damage is dispatched first")
        }
        EntityData::ArmorStand(_) | EntityData::EndCrystal(_) => unreachable!("returned above"),
        EntityData::Boat(_) | EntityData::ChestBoat(_) => unreachable!("returned above"),
        EntityData::Minecart(_)
        | EntityData::FurnaceMinecart(_)
        | EntityData::ChestMinecart(_)
        | EntityData::HopperMinecart(_)
        | EntityData::CommandBlockMinecart(_)
        | EntityData::SpawnerMinecart(_)
        | EntityData::TntMinecart(_) => {
            unreachable!("returned above")
        }
        EntityData::GeneralLiving(_) => unreachable!("returned above"),
        EntityData::Marker(_)
        | EntityData::Interaction(_)
        | EntityData::BlockDisplay(_)
        | EntityData::ItemDisplay(_)
        | EntityData::TextDisplay(_)
        | EntityData::LargeFireball(_)
        | EntityData::SmallFireball(_)
        | EntityData::DragonFireball(_)
        | EntityData::WitherSkull(_)
        | EntityData::WindCharge(_)
        | EntityData::BreezeWindCharge(_)
        | EntityData::AreaEffectCloud(_)
        | EntityData::OminousItemSpawner(_) => unreachable!("returned above"),
        EntityData::ShulkerBullet(_) => unreachable!("returned above"),
    }
}

pub fn removed_or_invulnerable_to_damage(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
) -> bool {
    entity.removal_reason().is_some()
        || entity.invulnerable()
            && !source.damage_type.bypasses_invulnerability
            && !source
                .causing_entity
                .is_some_and(StoredDamageCausingEntity::is_creative_player)
}

fn refused() -> EntityHurtMutation {
    EntityHurtMutation {
        outcome: StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        },
        emits_entity_damage: false,
        emits_entity_die: false,
        game_event_source_id: None,
        destroyed_drops: Vec::new(),
        removal_before_drops: false,
    }
}

fn fire_immune(
    entity: &Entity,
    items: &dyn PersistentItemStackHolderResolver,
) -> Result<bool, ServerWorldError> {
    fire_immune_with_item_query(entity, |stack, damage_type| {
        Ok(!item_resists_damage(stack, damage_type, items))
    })
}

pub(crate) fn fire_immune_with_item_query<E>(
    entity: &Entity,
    can_be_hurt_by: impl FnOnce(&RuntimeItemStack, &'static str) -> Result<bool, E>,
) -> Result<bool, E> {
    if let EntityData::Item(item) = entity.data() {
        if !can_be_hurt_by(item.item(), "minecraft:in_fire")? {
            return Ok(true);
        }
    }
    Ok(entity.fire_immune())
}

/// Java's `(int)(intHealth - floatDamage)` arithmetic.
///
/// Rust's float-to-int cast has the same narrowing edges needed here: truncate
/// toward zero, NaN to zero, and infinities/out-of-range values saturating to
/// the signed endpoint.
fn java_health_after_damage(health: i32, damage: f32) -> i32 {
    (health as f32 - damage) as i32
}

#[cfg(test)]
pub mod tests;
