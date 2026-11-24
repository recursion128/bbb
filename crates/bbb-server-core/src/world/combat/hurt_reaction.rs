//! What `LivingEntity.hurtServer` does to a session past its health: the
//! direct entity it reads, the `markHurt`, `knockback` and `indicateDamage` of
//! a full hit (`LivingEntity.java:1214-1243`).

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::projectile_hit::mth_atan2;
use crate::entity::{Entity, EntityData};
use crate::world::LevelId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{HurtAnimation, InteractionHand, Vec3d};
use bbb_server_datapack::damage::DamageTypeFacts;
use bbb_server_types::geometry::Vec3Ops;
use bbb_server_types::{Result, ServerWorldError};

use crate::commands::CommandDamageSourceEntity;
use crate::world::{send, PlayerState, ServerWorld, WorldEvent};

/// `LivingEntity.LIVING_ENTITY_FLAG_IS_USING` and `LIVING_ENTITY_FLAG_OFF_HAND`
/// (`LivingEntity.java:175-176`) on a stored body.
const IS_USING_OFF_HAND: i8 = 1 | 2;

/// `DamageSource.getDirectEntity()` as `hurtServer` and `applyItemBlocking`
/// read it.
#[derive(Debug, Clone, Default)]
pub(in crate::world) struct HurtDirectEntity {
    /// `instanceof LivingEntity`.
    pub(in crate::world) living: Option<LivingAttacker>,
    /// `instanceof Projectile`.
    pub(in crate::world) projectile: Option<ProjectileKnockback>,
    /// `instanceof AbstractArrow`: `getPierceLevel()`.
    pub(in crate::world) arrow_pierce_level: Option<i8>,
}

#[derive(Debug, Clone)]
pub(in crate::world) struct LivingAttacker {
    pub(in crate::world) position: Vec3d,
    pub(in crate::world) weapon_source: BlockingWeaponSource,
    pub(in crate::world) blocked_by_item: BlockedByItem,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::world) enum BlockingWeaponSource {
    Player(ConnectionId),
    Stored(i32),
}

/// The `blockedByItem` a blocking session meets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::world) enum BlockedByItem {
    /// `LivingEntity.blockedByItem` (`LivingEntity.java:1357-1359`).
    Knockback,
    /// `Ravager.blockedByItem` (`Ravager.java:206-219`).
    Ravager,
    /// Adult `Hoglin` and `Zoglin` throw the defender (`Hoglin.java:108-112`,
    /// `Zoglin.java:184-188`).
    HoglinThrow,
}

/// `Projectile.calculateHorizontalHurtKnockbackDirection` (`Projectile.java:380-384`)
/// and the overrides that aim from the projectile's position instead
/// (`AbstractThrownPotion.java:124-128`, `FireworkRocketEntity.java:315-319`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::world) enum ProjectileKnockback {
    Motion(Vec3d),
    From(Vec3d),
}

impl ServerWorld {
    // RamTarget.java:77-91; DamageTypes.java:37.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn apply_goat_ram_player(
        &mut self,
        dimension: &LevelId,
        goat: uuid::Uuid,
        player: uuid::Uuid,
        origin: Vec3d,
        damage: f32,
        knockback: f64,
        direction: Vec3d,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(connection) = self.connection_of(player).filter(|connection| {
            self.levels
                .player(*connection)
                .is_some_and(|player| player.dimension == *dimension && !player.avatar_removed)
        }) else {
            return;
        };
        let Some(goat) = self
            .level_if_present_by_id(dimension)
            .and_then(|level| level.entities.entity_id_for_uuid(goat))
            .and_then(|id| self.level_by_id(dimension).entities.get(id))
            .filter(|entity| {
                entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_GOAT_ID
            })
        else {
            return;
        };
        let goat_id = goat.id();
        let source_entity = crate::level_event::PendingHurtEntity::Stored { entity_id: goat_id };
        let Some(source) = self.pending_hurt_source_entity(dimension, source_entity.clone()) else {
            return;
        };
        if self.hurt_connected_player(
            connection,
            "minecraft:mob_attack_no_aggro",
            damage,
            Some(source.clone()),
            Some(source.clone()),
            Some([origin.x, origin.y, origin.z]),
            events,
        ) {
            if let Err(detail) = self.queued_post_attack_checked(
                dimension,
                super::enchanted_damage::SourceEntity::Session(connection),
                "minecraft:mob_attack_no_aggro",
                Some(source_entity.clone()),
                Some(source_entity),
                events,
            ) {
                tracing::warn!("{detail}");
            }
        }
        let registry = std::sync::Arc::clone(&self.cold.damage_types);
        let Some(damage_type) = registry
            .iter()
            .find(|damage_type| damage_type.resource_id == "minecraft:mob_attack")
        else {
            return;
        };
        let direct = self.hurt_direct_entity(dimension, Some(&source));
        let blocking = if damage > 0.0 {
            match self.player_item_blocking(connection, damage_type, Some(origin), &direct) {
                Ok(blocking) => blocking,
                Err(detail) => {
                    tracing::warn!("{detail}");
                    return;
                }
            }
        } else {
            None
        };
        let blocked = blocking
            .as_ref()
            .map_or(0.0, |blocking| blocking.resolve(damage));
        if let Some(blocking) = blocking {
            self.apply_item_blocking(connection, &blocking, blocked, damage_type, &direct, events);
        }
        self.player_knockback(
            connection,
            knockback * if blocked > 0.0 { 0.5 } else { 1.0 },
            direction.x,
            direction.z,
        );
    }

    pub(in crate::world) fn hurt_direct_entity(
        &self,
        dimension: &LevelId,
        direct: Option<&CommandDamageSourceEntity>,
    ) -> HurtDirectEntity {
        match direct {
            None => HurtDirectEntity::default(),
            Some(CommandDamageSourceEntity::Player { user, position, .. }) => HurtDirectEntity {
                living: self
                    .command_player_connection(user)
                    .map(|connection_id| LivingAttacker {
                        position: *position,
                        weapon_source: BlockingWeaponSource::Player(connection_id),
                        blocked_by_item: BlockedByItem::Knockback,
                    }),
                ..HurtDirectEntity::default()
            },
            Some(CommandDamageSourceEntity::Stored { entity_id, .. }) => self
                .level_by_id(dimension)
                .entities
                .get(*entity_id)
                .map(stored_direct_entity)
                .unwrap_or_default(),
        }
    }

    pub(in crate::world) fn attacker_disable_blocking_seconds(
        &self,
        dimension: &LevelId,
        attacker: &LivingAttacker,
    ) -> Result<f32> {
        match attacker.weapon_source {
            BlockingWeaponSource::Player(connection_id) => {
                player_seconds_to_disable_blocking(self.levels.player(connection_id).ok_or(
                    ServerWorldError::MissingCatalogFact("original blocking source"),
                )?)
            }
            BlockingWeaponSource::Stored(entity_id) => {
                let Some(entity) = self.level_by_id(dimension).entities.get(entity_id) else {
                    return Ok(0.0);
                };
                // Warden.java:174-175.
                if entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID
                {
                    return Ok(5.0);
                }
                let Some(living) = entity.living() else {
                    return Ok(0.0);
                };
                crate::entity::living::active_main_hand_disable_blocking_seconds(
                    living.living_entity_flags() & IS_USING_OFF_HAND == IS_USING_OFF_HAND,
                    living
                        .equipment()
                        .get(EquipmentSlot::MainHand)
                        .map(|stack| stack.summary()),
                )
                .map_err(ServerWorldError::InvalidConfiguration)
            }
        }
    }

    /// `LivingEntity.knockback` (`LivingEntity.java:1612-1630`) for a session.
    pub(in crate::world) fn player_knockback(
        &mut self,
        connection_id: ConnectionId,
        power: f64,
        xd: f64,
        zd: f64,
    ) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let resistance = player
            .attributes
            .value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
            .unwrap_or(0.0);
        let Some(power) = crate::entity::living::scaled_knockback_power(power, resistance) else {
            return;
        };
        player.needs_sync = true;
        let (delta, on_ground) = (player.delta_movement(), player.on_ground());
        let random = &self.random;
        let delta = crate::entity::living::knockback_delta(delta, on_ground, power, xd, zd, || {
            random.next_double()
        });
        player.set_delta_movement(delta);
    }

    /// `IronGolem.doHurtTarget`'s toss (`IronGolem.java:195-197`) on a session.
    pub(in crate::world) fn player_lift(&mut self, connection_id: ConnectionId, y: f64) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let resistance = player
            .attributes
            .value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
            .unwrap_or(0.0);
        let delta = player.delta_movement();
        player.set_delta_movement(Vec3d {
            y: delta.y + y * (1.0 - resistance).max(0.0),
            ..delta
        });
    }

    pub(in crate::world) fn player_hoglin_throw(
        &mut self,
        connection_id: ConnectionId,
        from: Vec3d,
        power: f64,
    ) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let resistance = player
            .attributes
            .value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
            .unwrap_or(0.0);
        if let Some(push) = crate::entity::hoglin::throw_impulse(
            from,
            player.position(),
            power,
            resistance,
            &self.random,
        ) {
            player.set_delta_movement(player.delta_movement().plus(push));
            player.needs_sync = true;
            player.hurt_marked = true;
        }
    }

    /// `hurtServer`'s full-hit arm past the damage event (`:1222-1242`):
    /// `markHurt`, the knockback away from the source and, unless blocked,
    /// `ServerPlayer.indicateDamage` (`ServerPlayer.java:2115-2118`).
    ///
    /// A level without a tag snapshot cannot answer `NO_IMPACT` or
    /// `NO_KNOCKBACK`, and the arm it guards is left undone.
    pub(in crate::world) fn player_hurt_impact(
        &mut self,
        connection_id: ConnectionId,
        damage_type: &DamageTypeFacts,
        source_position: Option<Vec3d>,
        direct: &HurtDirectEntity,
        (blocked, damage): (bool, f32),
        events: &mut Vec<WorldEvent>,
    ) {
        let no_impact =
            self.player_damage_type_tag(connection_id, damage_type, "minecraft:no_impact");
        if no_impact == Some(false) && (!blocked || damage > 0.0) {
            self.levels[connection_id].hurt_marked = true;
        }
        let no_knockback =
            self.player_damage_type_tag(connection_id, damage_type, "minecraft:no_knockback");
        if no_knockback != Some(false) {
            return;
        }
        let player = self.levels.player_at(connection_id);
        let (position, y_rot, entity_id) = (
            player.position(),
            player.rotation().y_rot,
            player.entity_id(),
        );
        let (projectile_motion, projectile_origin) = match direct.projectile {
            Some(ProjectileKnockback::Motion(motion)) => (Some(motion), None),
            Some(ProjectileKnockback::From(from)) => (None, Some(from)),
            None => (None, None),
        };
        let (xd, zd) = crate::entity::living::hurt_knockback_direction(
            position,
            projectile_motion,
            projectile_origin,
            source_position,
        );
        self.player_knockback(connection_id, 0.4, xd, zd);
        if !blocked {
            let hurt_dir = (mth_atan2(zd, xd) * 180.0 / f64::from(std::f32::consts::PI)
                - f64::from(y_rot)) as f32;
            events.push(send(
                connection_id,
                ClientboundPlayPacket::HurtAnimation(HurtAnimation {
                    id: entity_id,
                    yaw: hurt_dir,
                }),
            ));
        }
    }

    /// `source.is(tag)` against the tag snapshot of the session's level;
    /// `None` where the level carries none.
    pub(in crate::world) fn player_damage_type_tag(
        &self,
        connection_id: ConnectionId,
        damage_type: &DamageTypeFacts,
        tag: &str,
    ) -> Option<bool> {
        Some(self.player_registry_tags(connection_id)?.contains(
            "minecraft:damage_type",
            tag,
            &damage_type.resource_id,
        ))
    }
}

/// `getSecondsToDisableBlocking` for a session: its main-hand weapon counts
/// only while it is also `getActiveItem()`, which an off-hand use is not.
fn player_seconds_to_disable_blocking(player: &PlayerState) -> Result<f32> {
    let using = &player.using_item;
    let selected = player.inventory.selected_item()?;
    crate::entity::living::active_main_hand_disable_blocking_seconds(
        using.is_using_item() && using.used_item_hand() == InteractionHand::OffHand,
        Some(&selected),
    )
    .map_err(ServerWorldError::InvalidConfiguration)
}

fn stored_direct_entity(entity: &Entity) -> HurtDirectEntity {
    let projectile = entity.is_projectile_class().then(|| match entity.data() {
        EntityData::SplashPotion(_)
        | EntityData::LingeringPotion(_)
        | EntityData::FireworkRocket(_) => ProjectileKnockback::From(entity.position()),
        _ => ProjectileKnockback::Motion(entity.delta_movement()),
    });
    let arrow_pierce_level = match entity.data() {
        EntityData::Arrow(arrow) => Some(arrow.pierce_level()),
        EntityData::SpectralArrow(arrow) => Some(arrow.pierce_level()),
        EntityData::Trident(trident) => Some(trident.pierce_level()),
        _ => None,
    };
    let living = entity.living().map(|_| LivingAttacker {
        position: entity.position(),
        weapon_source: BlockingWeaponSource::Stored(entity.id()),
        blocked_by_item: match entity.entity_type_id() {
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_RAVAGER_ID => BlockedByItem::Ravager,
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOGLIN_ID
            | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
                BlockedByItem::HoglinThrow
            }
            _ => BlockedByItem::Knockback,
        },
    });
    HurtDirectEntity {
        living,
        projectile,
        arrow_pierce_level,
    }
}
