#[cfg(test)]
use crate::block_getter::BlockGetter;
use crate::random::GameRandom;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_EXPLOSION_KNOCKBACK_RESISTANCE_ID;
use bbb_protocol::block_types::VANILLA_BLOCK_CREAKING_HEART_ID;
use bbb_protocol::game_rules::GameRuleValue;
use bbb_protocol::packets::{BlockPos, ParticlePayload, SoundEventHolder, Vec3d};
use bbb_protocol::particle_types::{
    VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID, VANILLA_PARTICLE_TYPE_EXPLOSION_ID,
    VANILLA_PARTICLE_TYPE_POOF_ID, VANILLA_PARTICLE_TYPE_SMOKE_ID,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use rustc_hash::FxHashSet;
use std::sync::Arc;

use crate::chunk::ChunkStore;
use crate::entity::block_drops::{pop_resources, PendingPopResource};
use crate::entity::damage::{
    StoredDamageCausingEntity, StoredEntityDamageContext, StoredEntityDamageRules,
    StoredEntityDamageSource,
};
use crate::entity::{spawn_chain_reaction_tnt, Entity, EntityData, EntityStore};
#[cfg(test)]
use crate::explosion::entity_hit_with_profile;
use crate::explosion::{
    entity_hit_with_profile_and_shapes, hurt_entities_box, is_small,
    should_affect_blocklike_entities, shuffle, BlockInteraction, ExplosionDamageProfile,
    ExplosionDamageSource, ExplosionGameRules, ExplosionInteraction, ExplosionPresentation,
    ExplosionSource, ExplosionSourceEntity, PendingExplosion, EXPLOSION_PACKET_RANGE_SQR,
};
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::level_event::VanillaGameEvent;
use crate::loot_runtime::{LootEntityFacts, LootEntityPosition};
use crate::neighbor_updater::UPDATE_LIMIT;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::Vec3Ops;

use super::creaking_heart_break::HeartXpPlayer;
use super::simulation_level::pending_spawns::PendingEntitySpawns;
use super::{send, LevelCtx, LevelId, ServerWorld, WorldEvent};
use crate::connection::{ClientboundPlayPacket, ConnectionId};

/// `SoundEvents.GENERIC_EXPLODE` (`Level.java:590`).
const EXPLOSION_SOUND: &str = "minecraft:entity.generic.explode";

/// `AbstractWindCharge.EXPLOSION_DAMAGE_CALCULATOR`, the knockback-only gust a
/// `WindChargedMobEffect` death sets off (`WindChargedMobEffect.java:24-38`).
pub(super) fn wind_charged_burst_profile() -> ExplosionDamageProfile {
    ExplosionDamageProfile::wind_charge(false, 1.0)
}

/// `ServerExplosion.damageSource`: the custom source, else `badRespawnPointExplosion`
/// or `Explosion.getDefaultDamageSource`.
fn explosion_damage_type<'a>(
    source: Option<&ExplosionSource>,
    custom: Option<&'a ExplosionDamageSource>,
    respawn_point: bool,
) -> &'a str {
    if let Some(custom) = custom {
        custom.damage_type.as_str()
    } else if respawn_point {
        "minecraft:bad_respawn_point"
    } else if source.is_some_and(|source| source.causing.is_some()) {
        "minecraft:player_explosion"
    } else {
        "minecraft:explosion"
    }
}

fn drops_from_explosion(block_state_id: i32) -> bool {
    !is_tnt_block(block_state_id)
}

/// Whether the block is `Blocks.TNT`, whose `onExplosionHit` drops nothing but
/// still spawns a chain-reaction primed TNT (`TntBlock.wasExploded`).
fn is_tnt_block(block_state_id: i32) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_TNT_ID)
}

#[cfg(test)]
struct ExplosionBlocks<'a> {
    chunks: &'a ChunkStore,
}

#[cfg(test)]
impl BlockGetter for ExplosionBlocks<'_> {
    fn block_state(&self, pos: BlockPos) -> i32 {
        super::stored_block_state(self.chunks, pos)
    }
}

#[derive(Clone, Copy)]
struct RespawnPointExplosion {
    water_shield: Option<BlockPos>,
}

/// `remainingPower -= 0.22500001F` (`ServerExplosion.java:162`).
const RAY_POWER_STEP: f32 = 0.225_000_01;

/// `float stepSize = 0.3F` (`ServerExplosion.java:140`), widened to a double
/// where the ray advances (`:159-161`).
const RAY_STEP: f32 = 0.3;

/// `calculateExplodedPositions`' answer, or why it could not be given.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExplodedPositions {
    positions: Vec<BlockPos>,
    /// Set when a ray crossed a block whose explosion resistance the level
    /// could not answer; `positions` is then empty.
    refusal: Option<String>,
}

fn calculate_exploded_positions(
    chunks: &ChunkStore,
    random: &GameRandom,
    water_shield: Option<BlockPos>,
    center: Vec3d,
    radius: f32,
    profile: &ExplosionDamageProfile,
) -> ExplodedPositions {
    let mut inserted: Vec<BlockPos> = Vec::new();
    let mut seen: FxHashSet<(i32, i32, i32)> = FxHashSet::default();
    let mut refusal = None;
    for xx in 0..16_i32 {
        for yy in 0..16_i32 {
            for zz in 0..16_i32 {
                if xx != 0 && xx != 15 && yy != 0 && yy != 15 && zz != 0 && zz != 15 {
                    continue;
                }
                let mut xd = f64::from(xx as f32 / 15.0 * 2.0 - 1.0);
                let mut yd = f64::from(yy as f32 / 15.0 * 2.0 - 1.0);
                let mut zd = f64::from(zz as f32 / 15.0 * 2.0 - 1.0);
                let d = (xd * xd + yd * yd + zd * zd).sqrt();
                xd /= d;
                yd /= d;
                zd /= d;
                let mut remaining_power: f32 = radius * (0.7 + random.next_float() * 0.6);
                let mut xp = center.x;
                let mut yp = center.y;
                let mut zp = center.z;
                type CachedVoxel = (BlockPos, Result<Option<f32>, ()>, i32, bool);
                let mut last_voxel: Option<CachedVoxel> = None;
                while remaining_power > 0.0 {
                    let pos = containing(xp, yp, zp);
                    let (resistance, block, can_explode) = match last_voxel {
                        Some((cached_pos, res, block, explode)) if cached_pos == pos => {
                            (res, block, explode)
                        }
                        _ => {
                            if !super::is_in_world_bounds(chunks, pos) {
                                break;
                            }
                            let block = super::stored_block_state(chunks, pos);
                            let res = block_explosion_resistance_with_profile_at(
                                chunks,
                                water_shield,
                                pos,
                                block,
                                profile,
                            );
                            let explode =
                                should_block_explode_with_profile(chunks, pos, block, profile);
                            last_voxel = Some((pos, res, block, explode));
                            (res, block, explode)
                        }
                    };
                    match resistance {
                        Ok(Some(resistance)) => remaining_power -= (resistance + 0.3) * 0.3,
                        Ok(None) => {}
                        Err(()) => {
                            refusal.get_or_insert_with(|| {
                                format!(
                                    "block state {block} has no explosion resistance fact, so \
                                     the exploded positions cannot be calculated"
                                )
                            });
                        }
                    }
                    if remaining_power > 0.0 && can_explode && seen.insert((pos.x, pos.y, pos.z)) {
                        inserted.push(pos);
                    }
                    xp += xd * f64::from(RAY_STEP);
                    yp += yd * f64::from(RAY_STEP);
                    zp += zd * f64::from(RAY_STEP);
                    remaining_power -= RAY_POWER_STEP;
                }
            }
        }
    }
    if refusal.is_some() {
        return ExplodedPositions {
            positions: Vec::new(),
            refusal,
        };
    }
    ExplodedPositions {
        positions: inserted,
        refusal: None,
    }
}

fn block_explosion_resistance_with_profile_at(
    chunks: &ChunkStore,
    water_shield: Option<BlockPos>,
    pos: BlockPos,
    block_state_id: i32,
    profile: &ExplosionDamageProfile,
) -> Result<Option<f32>, ()> {
    if water_shield == Some(pos) {
        if let Some(resistance) = bbb_protocol::block_types::vanilla_block_explosion_resistance(
            bbb_protocol::block_types::VANILLA_BLOCK_WATER_ID,
        ) {
            return Ok(Some(resistance));
        }
    }
    if matches!(profile, ExplosionDamageProfile::PrimedTntUsedPortal)
        && block_state_is_nether_portal(block_state_id)
    {
        return Ok(None);
    }
    if matches!(
        profile,
        ExplosionDamageProfile::TntMinecart { primed: true }
    ) && tnt_minecart_protects_rail(chunks, pos, block_state_id)
    {
        return Ok(Some(0.0));
    }
    if let ExplosionDamageProfile::Simple {
        immune_blocks: Some(immune),
        ..
    } = profile
    {
        let block_type_id =
            bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
                .or_else(|| {
                    chunks
                        .content_registry()
                        .block_by_state_id(block_state_id)
                        .map(|(block, _)| block.block_id)
                });
        return Ok(block_type_id
            .filter(|id| immune.contains(id))
            .map(|_| 3_600_000.0));
    }
    let resistance = crate::explosion::block_explosion_resistance_with_registry(
        block_state_id,
        chunks.content_registry(),
    )?;
    // `WitherBoss.canDestroy`: not air and not `#wither_immune`.
    if matches!(profile, ExplosionDamageProfile::DangerousWitherSkull)
        && !block_state_is_air(block_state_id)
        && !chunks
            .block_tags()
            .block_state_in("minecraft:wither_immune", block_state_id)
    {
        return Ok(resistance.map(|resistance| resistance.min(0.8)));
    }
    Ok(resistance)
}

fn should_block_explode_with_profile(
    chunks: &ChunkStore,
    pos: BlockPos,
    block_state_id: i32,
    profile: &ExplosionDamageProfile,
) -> bool {
    !matches!(
        profile,
        ExplosionDamageProfile::Simple {
            explodes_blocks: false,
            ..
        }
    ) && !(matches!(profile, ExplosionDamageProfile::PrimedTntUsedPortal)
        && block_state_is_nether_portal(block_state_id))
        && (!matches!(
            profile,
            ExplosionDamageProfile::TntMinecart { primed: true }
        ) || !tnt_minecart_protects_rail(chunks, pos, block_state_id))
}

fn block_state_is_nether_portal(block_state_id: i32) -> bool {
    bbb_protocol::block_state_types::vanilla_block_state_block_type_id(block_state_id)
        == Some(bbb_protocol::block_types::VANILLA_BLOCK_NETHER_PORTAL_ID)
}

fn tnt_minecart_protects_rail(chunks: &ChunkStore, pos: BlockPos, block_state_id: i32) -> bool {
    let tags = chunks.block_tags();
    tags.block_state_in(RAILS_TAG, block_state_id)
        || tags.block_state_in(
            RAILS_TAG,
            super::stored_block_state(
                chunks,
                BlockPos {
                    x: pos.x,
                    y: pos.y.wrapping_add(1),
                    z: pos.z,
                },
            ),
        )
}

const RAILS_TAG: &str = "minecraft:rails";

fn containing(x: f64, y: f64, z: f64) -> BlockPos {
    BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    }
}

impl ServerWorld {
    pub(crate) fn explode(
        &mut self,
        dimension: impl Into<LevelId>,
        request: PendingExplosion,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let dimension = dimension.into();
        self.explode_with_respawn_point(dimension, request, None, events)
    }

    pub(in crate::world) fn explode_bad_respawn_point(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        water_shield: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let request = PendingExplosion {
            source: None,
            center: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            radius: 5.0,
            fire: true,
            interaction: ExplosionInteraction::Block,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        };
        let _ = self.explode_with_respawn_point(
            dimension,
            request,
            Some(RespawnPointExplosion {
                water_shield: water_shield.then_some(pos),
            }),
            events,
        );
    }

    fn explode_with_respawn_point(
        &mut self,
        dimension: LevelId,
        request: PendingExplosion,
        respawn_point: Option<RespawnPointExplosion>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        self.explode_with_respawn_point_inner(dimension, request, respawn_point, events)
    }

    fn explode_with_respawn_point_inner(
        &mut self,
        dimension: LevelId,
        request: PendingExplosion,
        respawn_point: Option<RespawnPointExplosion>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        // The lifecycle drain consumes the removed source before block loot runs.
        let removed_block_source = request.source.as_ref().and_then(|source| {
            if !matches!(
                request
                    .interaction
                    .block_interaction(self.explosion_game_rules()),
                BlockInteraction::Destroy | BlockInteraction::DestroyWithDecay
            ) {
                return None;
            }
            let entities = &self.level_by_id(&dimension).entities;
            let entity = entities.removed_entity(source.direct.entity_id, source.direct.uuid)?;
            explosion_source_loot_facts(entities, entity, &source.direct)
        });
        self.apply_level_entity_lifecycle_by_key(&dimension, events);

        let center = request.center;
        let radius = request.radius;
        let source = request.source.as_ref();
        let rules = self.explosion_game_rules();
        let block_interaction = request.interaction.block_interaction(rules);

        // `:236`: `this.level.gameEvent(this.source, GameEvent.EXPLODE, this.center)`.
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event_with_source_at(
                VanillaGameEvent::EXPLODE,
                center,
                source.map(|source| source.direct.entity_id),
                None,
            );
        self.post_pending_game_events(&dimension);

        // `:237`: `calculateExplodedPositions()`.
        let sampled = if matches!(
            request.damage_profile,
            ExplosionDamageProfile::Simple {
                explodes_blocks: false,
                ..
            }
        ) {
            ExplodedPositions {
                positions: Vec::new(),
                refusal: None,
            }
        } else {
            let level = self.level_mut_by_id(&dimension);
            calculate_exploded_positions(
                &level.chunks,
                &level.random,
                respawn_point.and_then(|point| point.water_shield),
                center,
                radius,
                &request.damage_profile,
            )
        };
        if let Some(detail) = sampled.refusal {
            tracing::warn!("{detail}");
        }
        let mut to_blow = sampled.positions;
        let block_count = i32::try_from(to_blow.len()).unwrap_or(i32::MAX);

        // `:238`: `hurtEntities()`.
        let hit_players = self.hurt_explosion_entities(
            dimension,
            source,
            removed_block_source.as_ref(),
            center,
            radius,
            block_interaction,
            &request.damage_profile,
            rules,
            request.damage_source.as_ref(),
            respawn_point,
            events,
        )?;

        // `:239-244`: `if (this.interactsWithBlocks()) interactWithBlocks(toBlow)`.
        if block_interaction != BlockInteraction::Keep && !to_blow.is_empty() {
            self.interact_with_blocks(
                dimension,
                &mut to_blow,
                block_interaction,
                radius,
                source,
                explosion_damage_type(
                    source,
                    request.damage_source.as_ref(),
                    respawn_point.is_some(),
                ),
                removed_block_source.as_ref(),
                events,
            );
        }

        if request.fire {
            self.create_explosion_fire(dimension, &to_blow);
        }

        self.apply_level_entity_lifecycle_by_key(&dimension, events);

        // `:1222-1229`: the `ClientboundExplodePacket` fan-out.
        self.broadcast_explosion_packet(
            dimension,
            center,
            radius,
            block_count,
            is_small(radius, block_interaction),
            request.presentation,
            &hit_players,
            events,
        );
        Some(())
    }

    #[allow(clippy::too_many_arguments)]
    fn hurt_explosion_entities(
        &mut self,
        dimension: LevelId,
        source: Option<&ExplosionSource>,
        removed_source: Option<&LootEntityFacts>,
        center: Vec3d,
        radius: f32,
        block_interaction: BlockInteraction,
        damage_profile: &ExplosionDamageProfile,
        rules: ExplosionGameRules,
        custom_damage_source: Option<&ExplosionDamageSource>,
        respawn_point: Option<RespawnPointExplosion>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<Vec<(ConnectionId, Vec3d)>> {
        let Some(box_to_hurt) = hurt_entities_box(center, radius) else {
            return Some(Vec::new());
        };
        let block_affects =
            should_affect_blocklike_entities(rules.mob_griefing, source, block_interaction);
        let source_id = source.map(|source| source.direct.entity_id);
        let source_in_water = source.is_some_and(|source| source.in_water);
        let stored_causing = source
            .and_then(|source| source.causing.as_ref())
            .map(stored_explosion_source);
        let redirect_owner = source
            .and_then(|source| source.causing.as_ref())
            .map(|causing| causing.uuid);

        let explicit_position = custom_damage_source.map_or_else(
            || respawn_point.map(|_| center),
            |source| source.explicit_position,
        );
        let damage_type_id =
            explosion_damage_type(source, custom_damage_source, respawn_point.is_some());
        let indirect_is_mob = source
            .and_then(|source| source.indirect.as_ref())
            .is_some_and(|indirect| indirect.is_mob());
        let registry_tags = self
            .level_by_id(&dimension)
            .chunks
            .registry_tags_shared()
            .map_or(
                crate::entity::EntityRegistryTagView::Builtin26_1,
                crate::entity::EntityRegistryTagView::Frozen,
            );
        let entities = &self.level_by_id(&dimension).entities;
        let mut candidate_ids = entities.ids_in_box_sections(box_to_hurt);
        // `Level.getEntities` also yields every dragon part in the box (`Level.java:777-781`).
        for dragon in entities.dragons() {
            if Some(dragon.id()) == source_id {
                continue;
            }
            candidate_ids.extend(
                crate::entity::ender_dragon::projectile_parts(dragon)
                    .filter(|(_, bounds, _)| bounds.intersects(box_to_hurt))
                    .map(|(id, _, _)| id),
            );
        }
        let damage_types = std::sync::Arc::clone(&self.cold.damage_types);
        let damage_type = damage_types
            .iter()
            .find(|entry| entry.resource_id == damage_type_id);
        let stored_rules = stored_explosion_damage_rules(
            rules.mob_griefing,
            self.game_rule_bool("minecraft:entity_drops", true),
            self.game_rule_bool("minecraft:tnt_explodes", true),
        );
        let context = StoredEntityDamageContext {
            block_drops: self.game_rule_bool("minecraft:block_drops", true),
            attacker_may_build: source
                .and_then(|source| source.causing.as_ref())
                .and_then(|causing| causing.player_facts())
                .is_none_or(|(_, may_build)| may_build),
            attacker_spectator: source.and_then(|source| source.causing.as_ref()).and_then(
                |causing| match causing.kind {
                    crate::explosion::ExplosionSourceEntityKind::Player { spectator, .. } => {
                        Some(spectator)
                    }
                    _ => None,
                },
            ),
        };

        for id in candidate_ids {
            if Some(id) == source_id {
                continue;
            }
            let hit = {
                let level = self.level_by_id(&dimension);
                let Some(entity) = level
                    .entities
                    .get(id)
                    .or_else(|| crate::entity::ender_dragon::part_entity(&level.entities, id))
                else {
                    continue;
                };
                if !entity.is_alive() || !entity.bounding_box().intersects(box_to_hurt) {
                    continue;
                }
                if ignore_explosion(
                    entity,
                    source_in_water,
                    block_affects,
                    indirect_is_mob,
                    rules.mob_griefing,
                ) {
                    continue;
                }
                // ServerExplosion.java:95: COLLIDER rayclip evaluates CollisionContext.of(entity).
                entity_hit_with_profile_and_shapes(
                    center,
                    radius,
                    entity.position(),
                    explosion_entity_origin(entity),
                    entity.bounding_box(),
                    entity
                        .living()
                        .and_then(|living| {
                            living
                                .attributes()
                                .value(VANILLA_ATTRIBUTE_EXPLOSION_KNOCKBACK_RESISTANCE_ID)
                        })
                        .unwrap_or(0.0),
                    damage_profile,
                    |pos| {
                        super::movement_collision::stored_body_collision_shape(
                            &level.chunks,
                            entity,
                            pos,
                            None,
                        )
                        .unwrap_or_else(|| {
                            super::movement_collision::block_collision_shape(
                                &level.chunks,
                                pos,
                                None,
                            )
                        })
                    },
                )
            };
            let Some(hit) = hit else {
                continue;
            };

            if damage_profile.damages_entities() {
                let Some(damage_type) = damage_type else {
                    continue;
                };
                let completed =
                    self.with_stored_hurt_level_checked(&dimension, None, |entities, level, _| {
                        crate::entity::projectile_action::hurt_stored_entity(
                            crate::entity::projectile_action::StoredHurtRequest {
                                direct_entity_snapshot: source.map(|source| &source.direct),
                                target_entity_id: id,
                                damage_type_resource_id: damage_type_id,
                                amount: hit.damage,
                                direct_resource_id: source
                                    .map(|source| source.direct.entity_resource_id.as_str()),
                                registry_tags: &registry_tags,
                                causing_entity_snapshot: source
                                    .and_then(|source| source.causing.clone()),
                                source_position: explicit_position
                                    .or_else(|| source.map(|source| source.direct.position)),
                                explicit_source_position: explicit_position,
                            },
                            None,
                            StoredEntityDamageSource {
                                damage_type,
                                direct_entity_id: source_id,
                                causing_entity: stored_causing,
                                attacker_views: Default::default(),
                            },
                            stored_rules,
                            context,
                            entities,
                            level,
                        )
                    });
                let _ = completed?;
            }
            // What the hurt set off -- a removed mob's effects, a crystal's own
            // blast -- runs before this explosion moves on.
            let _ = self.run_entity_commands(&dimension, events);

            let redirectable = self.level_by_id(&dimension).entities.get(id).map(|entity| {
                registry_tags.contains(
                    "minecraft:entity_type",
                    "minecraft:redirectable_projectile",
                    entity.data().entity_resource_id(),
                )
            });
            if hit.knockback != Vec3d::default() {
                let hanging = self
                    .level_by_id(&dimension)
                    .entities
                    .get(id)
                    .is_some_and(|entity| {
                        matches!(
                            entity.data(),
                            EntityData::Painting(_)
                                | EntityData::ItemFrame(_)
                                | EntityData::GlowItemFrame(_)
                                | EntityData::LeashKnot(_)
                        )
                    });
                if hanging {
                    if self
                        .with_stored_hurt_level(&dimension, |entities, level| {
                            entities.push_hanging_entity(level, id, hit.knockback)
                        })
                        .is_none()
                    {
                        continue;
                    }
                } else if let Some(entity) = self.level_mut_by_id(&dimension).entities.get_mut(id) {
                    if entity.is_alive() {
                        crate::entity::push(
                            entity,
                            hit.knockback.x,
                            hit.knockback.y,
                            hit.knockback.z,
                        );
                    }
                }
            }
            let level = self.level_mut_by_id(&dimension);
            if matches!(redirectable, Some(true)) {
                if let Some(entity) = level.entities.get_mut(id) {
                    let _ = crate::entity::hurting_projectile::set_owner_from_explosion(
                        entity,
                        redirect_owner,
                    );
                }
            }
        }

        let player_ids: Vec<_> = self
            .levels
            .player_entries()
            .filter_map(|(&connection_id, player)| {
                (player.dimension == dimension
                    && !player.avatar_removed
                    && player.game_type != bbb_protocol::game_types::GameType::Spectator
                    && Some(player.entity_id()) != source_id)
                    .then_some(connection_id)
            })
            .collect();
        let causing = source
            .and_then(|source| source.causing.as_ref())
            .and_then(|source| self.explosion_command_source(source));
        let direct = source.and_then(|source| self.explosion_command_source(&source.direct));
        let mut hit_players = Vec::new();
        let mut retained_source_nbt = None;
        for connection_id in player_ids {
            let Some((hit, direction)) = (|| {
                let player = self.levels.player(connection_id)?;
                let level = self.level_by_id(&dimension);
                let origin = player.eye_position().ok()?;
                let hit = entity_hit_with_profile_and_shapes(
                    center,
                    radius,
                    player.position(),
                    origin,
                    bbb_server_types::interaction::player_pose_bounding_box(
                        player.position(),
                        player.pose_id,
                        player.pose_scale(),
                    )
                    .ok()?,
                    0.0,
                    damage_profile,
                    |pos| {
                        super::movement_collision::player_block_collision_shape_checked(
                            &level.chunks,
                            player,
                            pos,
                        )
                        .or_else(|| {
                            super::movement_collision::block_collision_shape_checked(
                                &level.chunks,
                                pos,
                                None,
                            )
                        })
                        .unwrap_or_else(|| {
                            bbb_server_types::geometry::empty_context_block_collision_boxes(
                                super::stored_block_state(&level.chunks, pos),
                                pos,
                            )
                            .unwrap_or_else(|| {
                                vec![bbb_server_types::geometry::Aabb::of_block(BlockPos {
                                    x: 0,
                                    y: 0,
                                    z: 0,
                                })]
                            })
                        })
                    },
                )?;
                Some((
                    hit,
                    Vec3d {
                        x: origin.x - center.x,
                        y: origin.y - center.y,
                        z: origin.z - center.z,
                    }
                    .normalize(),
                ))
            })() else {
                continue;
            };
            let keep_source_nbt = source
                .is_some_and(|source| source.direct.entity_resource_id == "minecraft:wind_charge")
                || self.levels[connection_id]
                    .cold
                    .fall_impulse
                    .impact_position
                    .is_some();
            let source_nbt = if keep_source_nbt {
                retained_source_nbt
                    .get_or_insert_with(|| {
                        let source = &source?.direct;
                        let entities = &self.level_by_id(&dimension).entities;
                        if source.entity_resource_id == "minecraft:player" {
                            entities.player_by_uuid(source.uuid).filter(|player| {
                                player.entity_id() == source.entity_id
                                    && player.dimension == dimension
                            })?;
                            return self.player_loot_nbt(source.uuid).ok().map(Arc::new);
                        }
                        entities
                            .get(source.entity_id)
                            .and_then(|entity| {
                                explosion_source_loot_facts(entities, entity, source)
                            })
                            .and_then(|facts| facts.nbt)
                            .or_else(|| removed_source.and_then(|facts| facts.nbt.clone()))
                    })
                    .clone()
            } else {
                None
            };
            if damage_profile.damages_entities() {
                self.hurt_connected_player_checked(
                    connection_id,
                    damage_type_id,
                    hit.damage,
                    causing.clone(),
                    direct.clone(),
                    explicit_position.map(|p| [p.x, p.y, p.z]),
                    events,
                )?;
            }
            let player = self.levels.player_mut(connection_id).unwrap();
            // Death rewards can synchronously change attributes and game mode during hurt.
            let Some(resistance) = player
                .attributes
                .value(VANILLA_ATTRIBUTE_EXPLOSION_KNOCKBACK_RESISTANCE_ID)
            else {
                continue;
            };
            let simple_flying = matches!(damage_profile, ExplosionDamageProfile::Simple { .. })
                && player.abilities.flying;
            let multiplier = if simple_flying {
                0.0
            } else {
                damage_profile.knockback_multiplier()
            };
            let power = (1.0 - hit.dist)
                * f64::from(hit.exposure)
                * f64::from(multiplier)
                * (1.0 - resistance);
            let knockback = direction.scale(power);
            if !simple_flying
                && knockback.x.is_finite()
                && knockback.y.is_finite()
                && knockback.z.is_finite()
            {
                player.set_delta_movement(player.delta_movement().plus(knockback));
                player.needs_sync = true;
            }
            if player.game_type != bbb_protocol::game_types::GameType::Spectator
                && !simple_flying
                && (player.game_type != bbb_protocol::game_types::GameType::Creative
                    || !player.abilities.flying)
            {
                hit_players.push((connection_id, knockback));
            }
            player.on_explosion_impulse(source.map(|source| &source.direct), source_nbt);
        }
        Some(hit_players)
    }

    fn explosion_command_source(
        &self,
        source: &crate::explosion::ExplosionSourceEntity,
    ) -> Option<crate::commands::CommandDamageSourceEntity> {
        match source.kind {
            crate::explosion::ExplosionSourceEntityKind::Player {
                instabuild,
                may_build,
                spectator,
            } => {
                let name = self
                    .levels
                    .players()
                    .find(|player| player.profile.uuid == source.uuid)
                    .map_or_else(
                        || source.uuid.to_string(),
                        |player| player.profile.name.clone(),
                    );
                Some(crate::commands::CommandDamageSourceEntity::Player {
                    user: crate::commands::CommandPlayerTarget {
                        profile: crate::admin::NameAndId {
                            name: name.into(),
                            id: source.uuid,
                        },
                    },
                    entity_id: source.entity_id,
                    position: source.position,
                    instabuild,
                    may_build,
                    spectator,
                    display_name: crate::commands::translatable(
                        "entity.minecraft.player",
                        Vec::new(),
                    ),
                })
            }
            crate::explosion::ExplosionSourceEntityKind::Mob
            | crate::explosion::ExplosionSourceEntityKind::LivingOther
            | crate::explosion::ExplosionSourceEntityKind::Other => {
                let (namespace, path) = source.entity_resource_id.split_once(':')?;
                Some(crate::commands::CommandDamageSourceEntity::Stored {
                    entity_id: source.entity_id,
                    entity_resource_id: source.entity_resource_id.clone(),
                    uuid: Some(source.uuid),
                    position: Some(source.position),
                    is_mob: source.is_mob(),
                    is_living_non_player: source.is_living_non_player(),
                    display_name: crate::commands::translatable(
                        format!("entity.{namespace}.{path}"),
                        Vec::new(),
                    ),
                })
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn interact_with_blocks(
        &mut self,
        dimension: LevelId,
        to_blow: &mut [BlockPos],
        block_interaction: BlockInteraction,
        radius: f32,
        source: Option<&ExplosionSource>,
        damage_type_id: &str,
        removed_source: Option<&LootEntityFacts>,
        events: &mut Vec<WorldEvent>,
    ) {
        shuffle(to_blow, &self.random);
        if block_interaction == BlockInteraction::TriggerBlock {
            let can_trigger = source.is_none_or(|source| {
                source.direct.entity_resource_id != "minecraft:breeze_wind_charge"
                    || self.game_rule_bool("minecraft:mob_griefing", true)
            });
            let mut bell_to_ring: Option<(BlockPos, bbb_protocol::packets::Direction)> = None;
            let mut beehive_to_anger: Option<BlockPos> = None;
            let mut idx = 0;
            while idx < to_blow.len() {
                self.with_level(dimension, |level| {
                    while idx < to_blow.len() {
                        let pos = to_blow[idx];
                        idx += 1;
                        let state = super::stored_block_state(level.chunks, pos);
                        if can_trigger {
                            if crate::block_behavior::bell::is_bell(state) {
                                bell_to_ring =
                                    Some((pos, crate::block_behavior::bell::facing(state)));
                                break;
                            }
                            crate::block_behavior::explosion::trigger_block(level, pos, state);
                        }
                        // `BeehiveBlock.java:111-115`: anger also follows trigger-only hits.
                        if crate::block_behavior::beehive::is_beehive(state) {
                            beehive_to_anger = Some(pos);
                            break;
                        }
                    }
                });
                if let Some((pos, facing)) = bell_to_ring.take() {
                    self.ring_bell(&dimension, pos, facing, None, events);
                }
                if let Some(pos) = beehive_to_anger.take() {
                    self.anger_bees_near_hive(dimension, pos);
                }
            }
            return;
        }

        let explosion_radius = if block_interaction == BlockInteraction::DestroyWithDecay {
            Some(radius)
        } else {
            None
        };
        let owner_is_player = source
            .and_then(|source| source.indirect.as_ref())
            .is_some_and(|indirect| {
                matches!(
                    indirect.kind,
                    crate::explosion::ExplosionSourceEntityKind::Player { .. }
                )
            });

        let mut collectors: Vec<PendingPopResource> = Vec::new();
        let air = self.level_by_id(&dimension).chunks.air_state_id();
        // `TntBlock.wasExploded` (`TntBlock.java:75-82`).
        let tnt_explodes = self.game_rule_bool("minecraft:tnt_explodes", true);
        let beehive_release_source = source.is_some_and(|source| {
            matches!(
                source.direct.entity_resource_id.as_str(),
                "minecraft:tnt"
                    | "minecraft:creeper"
                    | "minecraft:wither_skull"
                    | "minecraft:wither"
                    | "minecraft:tnt_minecart"
            )
        });
        let mut breaker_facts: Option<Option<LootEntityFacts>> = None;
        let compute_breaker_facts = |world: &ServerWorld| -> Option<LootEntityFacts> {
            let source = &source?.direct;
            let entities = &world.level_by_id(&dimension).entities;
            if source.entity_resource_id == "minecraft:player" {
                let player = entities.player_by_uuid(source.uuid).filter(|player| {
                    player.entity_id() == source.entity_id && player.dimension == dimension
                })?;
                return Some(world.loot_view_for_player(player));
            }
            let entity = entities
                .get(source.entity_id)
                .filter(|entity| entity.removal_reason().is_none())?;
            explosion_source_loot_facts(entities, entity, source)
        };
        let is_special_block = |state: i32| -> bool {
            let block_type =
                bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state);
            block_type == Some(VANILLA_BLOCK_CREAKING_HEART_ID)
                || crate::block_behavior::beehive::is_beehive(state)
                || (tnt_explodes && is_tnt_block(state))
                || (drops_from_explosion(state)
                    && (crate::block_behavior::infested::host_state_by_infested(state).is_some()
                        || (owner_is_player
                            && block_type
                                .and_then(bbb_server_types::experience::block_break_experience)
                                .is_some())))
        };
        let mut idx = 0;
        while idx < to_blow.len() {
            let pos = to_blow[idx];
            let block_state_id =
                super::stored_block_state(&self.level_by_id(&dimension).chunks, pos);
            if block_state_is_air(block_state_id) {
                idx += 1;
                continue;
            }
            if is_special_block(block_state_id) {
                idx += 1;
                if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(
                    block_state_id,
                ) == Some(VANILLA_BLOCK_CREAKING_HEART_ID)
                {
                    self.creaking_heart_explosion_hit(
                        dimension,
                        pos,
                        block_state_id,
                        source,
                        damage_type_id,
                    );
                }
                if drops_from_explosion(block_state_id) {
                    let facts = breaker_facts.get_or_insert_with(|| compute_breaker_facts(self));
                    let breaker = if source.is_none() {
                        crate::loot_runtime::LootEntity::Absent
                    } else {
                        facts.as_ref().or(removed_source).map_or(
                            crate::loot_runtime::LootEntity::Opaque,
                            crate::loot_runtime::LootEntity::Facts,
                        )
                    };
                    self.spawn_after_break(
                        dimension,
                        pos,
                        block_state_id,
                        &bbb_protocol::packets::ItemStackSummary::empty(),
                        owner_is_player,
                    );
                    if beehive_release_source
                        && crate::block_behavior::beehive::is_beehive(block_state_id)
                    {
                        self.release_beehive_explosion_occupants(
                            dimension,
                            pos,
                            block_state_id,
                            events,
                        );
                    }
                    self.with_explosion_block_loot_level(dimension, |level| {
                        level.collect_explosion_block_loot_at(
                            pos,
                            block_state_id,
                            explosion_radius,
                            breaker,
                            &mut collectors,
                        );
                    });
                }
                self.set_explosion_block(dimension, pos, air);
                // `BeehiveBlock.java:111-115`: the base hit releases occupants before anger.
                if crate::block_behavior::beehive::is_beehive(block_state_id) {
                    self.anger_bees_near_hive(dimension, pos);
                }
                if tnt_explodes && is_tnt_block(block_state_id) {
                    let owner = source
                        .and_then(|source| source.indirect.as_ref())
                        .map(|indirect| indirect.uuid);
                    let position = Vec3d {
                        x: f64::from(pos.x) + 0.5,
                        y: f64::from(pos.y),
                        z: f64::from(pos.z) + 0.5,
                    };
                    let level = self.level_mut_by_id(&dimension);
                    let _ = spawn_chain_reaction_tnt(
                        &mut level.entities,
                        &level.random,
                        position,
                        owner,
                    );
                }
                continue;
            }
            if breaker_facts.is_none() && drops_from_explosion(block_state_id) {
                breaker_facts = Some(compute_breaker_facts(self));
            }
            let breaker = if source.is_none() {
                crate::loot_runtime::LootEntity::Absent
            } else {
                breaker_facts
                    .as_ref()
                    .and_then(Option::as_ref)
                    .or(removed_source)
                    .map_or(
                        crate::loot_runtime::LootEntity::Opaque,
                        crate::loot_runtime::LootEntity::Facts,
                    )
            };
            self.with_level(dimension, |level| {
                while idx < to_blow.len() {
                    let pos = to_blow[idx];
                    let state = super::stored_block_state(level.chunks, pos);
                    if block_state_is_air(state) {
                        idx += 1;
                        continue;
                    }
                    if is_special_block(state) {
                        break;
                    }
                    let drops = drops_from_explosion(state);
                    if drops && breaker_facts.is_none() && source.is_some() {
                        break;
                    }
                    idx += 1;
                    if drops {
                        level.collect_explosion_block_loot_at(
                            pos,
                            state,
                            explosion_radius,
                            breaker,
                            &mut collectors,
                        );
                    }
                    level.set_block(pos, air, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
                }
            });
        }

        // `:222-224`: `popResource` for each merged stack, its three offsets
        // drawn off the level random after the per-position draws above.
        if !collectors.is_empty() {
            self.pop_explosion_resources(dimension, collectors);
        }
    }

    fn creaking_heart_explosion_hit(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        state: i32,
        source: Option<&ExplosionSource>,
        damage_type_id: &str,
    ) {
        if bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
            != Some(VANILLA_BLOCK_CREAKING_HEART_ID)
        {
            return;
        }
        let damage_type = self
            .cold
            .damage_types
            .iter()
            .find(|entry| entry.resource_id == damage_type_id)
            .cloned();
        let direct_id = source.map(|source| source.direct.entity_id);
        let causing = source
            .and_then(|source| source.causing.as_ref())
            .map(stored_explosion_source);
        let xp_player = source
            .and_then(|source| source.indirect.as_ref())
            .and_then(|indirect| {
                if let crate::explosion::ExplosionSourceEntityKind::Player {
                    instabuild,
                    spectator,
                    ..
                } = indirect.kind
                {
                    Some(HeartXpPlayer {
                        instabuild,
                        spectator,
                    })
                } else {
                    None
                }
            });
        self.creaking_heart_before_removal(
            dimension,
            pos,
            state,
            damage_type
                .as_ref()
                .map(|damage_type| StoredEntityDamageSource {
                    damage_type,
                    direct_entity_id: direct_id,
                    causing_entity: causing,
                    attacker_views: Default::default(),
                }),
            xp_player,
        );
    }

    fn create_explosion_fire(&mut self, dimension: LevelId, positions: &[BlockPos]) {
        if positions.is_empty() {
            return;
        }
        let random = self.random.clone();
        self.with_level(dimension, |level| {
            for &pos in positions {
                if random.next_int(3) != 0 {
                    continue;
                }
                if !block_state_is_air(super::stored_block_state(level.chunks, pos)) {
                    continue;
                }
                let below = BlockPos {
                    y: pos.y.wrapping_sub(1),
                    ..pos
                };
                if crate::block_behavior::support::solid_render_with_registry(
                    super::stored_block_state(level.chunks, below),
                    level.chunks.content_registry(),
                ) != Some(true)
                {
                    continue;
                }
                let state = {
                    let chunks = &*level.chunks;
                    let tags = chunks
                        .registry_tags()
                        .unwrap_or_else(|| crate::registry_tags::RegistryTagSnapshot::builtin());
                    crate::block_behavior::fire::state_for_placement(pos, tags, &mut |at| {
                        super::stored_block_state(chunks, at)
                    })
                };
                if let Some(state) = state {
                    level.set_block(pos, state, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
                } else {
                    tracing::warn!("explosion fire placement facts cannot be resolved");
                }
            }
        });
    }

    fn set_explosion_block(&mut self, dimension: LevelId, pos: BlockPos, state: i32) {
        self.with_level(dimension, |level| {
            level.set_block(pos, state, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
        });
    }

    fn pop_explosion_resources(
        &mut self,
        dimension: LevelId,
        mut collectors: Vec<PendingPopResource>,
    ) {
        self.with_explosion_level(dimension, |slice| {
            let LevelCtx {
                random,
                game_rules,
                item_spawns,
                ..
            } = &mut *slice;
            pop_resources(&mut collectors, random, game_rules.block_drops, item_spawns);
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn broadcast_explosion_packet(
        &self,
        dimension: LevelId,
        center: Vec3d,
        radius: f32,
        block_count: i32,
        small: bool,
        presentation: ExplosionPresentation,
        hit_players: &[(ConnectionId, Vec3d)],
        events: &mut Vec<WorldEvent>,
    ) {
        let (explosion_particle, explosion_sound, block_particles) = match presentation {
            ExplosionPresentation::Default => (
                ParticlePayload {
                    particle_type_id: if small {
                        VANILLA_PARTICLE_TYPE_EXPLOSION_ID
                    } else {
                        VANILLA_PARTICLE_TYPE_EXPLOSION_EMITTER_ID
                    },
                    raw_options: Vec::new(),
                },
                SoundEventHolder::Reference {
                    registry_id: vanilla_sound_event_registry_id(EXPLOSION_SOUND)
                        .expect("builtin explosion sound"),
                },
                default_explosion_block_particles(),
            ),
            ExplosionPresentation::Custom {
                small_particle,
                large_particle,
                block_particles,
                sound,
            } => (
                if small {
                    small_particle
                } else {
                    large_particle
                },
                sound,
                block_particles,
            ),
        };
        for (&connection_id, player) in self.levels.player_entries() {
            if player.dimension != dimension || player.avatar_removed {
                continue;
            }
            if player.position().distance_to_sqr(center) >= EXPLOSION_PACKET_RANGE_SQR {
                continue;
            }
            let player_knockback = hit_players
                .iter()
                .find(|(id, _)| *id == connection_id)
                .map(|(_, knockback)| *knockback);
            let packet = ClientboundPlayPacket::Explode(bbb_protocol::packets::Explosion {
                center,
                radius,
                block_count,
                player_knockback,
                explosion_particle: explosion_particle.clone(),
                explosion_sound: explosion_sound.clone(),
                block_particles: block_particles.clone(),
            });
            events.push(send(connection_id, packet));
        }
    }

    fn explosion_game_rules(&self) -> ExplosionGameRules {
        ExplosionGameRules {
            mob_griefing: self.game_rule_bool("minecraft:mob_griefing", true),
            block_explosion_drop_decay: self
                .game_rule_bool("minecraft:block_explosion_drop_decay", true),
            mob_explosion_drop_decay: self
                .game_rule_bool("minecraft:mob_explosion_drop_decay", true),
            tnt_explosion_drop_decay: self
                .game_rule_bool("minecraft:tnt_explosion_drop_decay", false),
        }
    }

    pub(in crate::world) fn game_rule_bool(&self, id: &str, default: bool) -> bool {
        match self.admin.game_rule_value(id) {
            Some(GameRuleValue::Boolean(value)) => value,
            Some(GameRuleValue::Integer(_)) | None => default,
        }
    }

    /// [`ServerWorld::with_level`] projected to the raw slice fields the
    /// explosion draws over, so a body can read blocks and the level random and
    /// collect drops in one borrow.
    fn with_explosion_level<R>(
        &mut self,
        dimension: LevelId,
        body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R,
    ) -> R {
        let players = self.nearby_players_by_key(&dimension);
        let mut leftover = PendingEntitySpawns::default();
        let answer =
            self.with_level_ctx(&dimension, &players, &mut leftover, |level, _| body(level));
        // The frame the explosion body ran on is over.
        self.post_pending_game_events(&dimension);
        self.apply_pending_spawns(&dimension, leftover);
        answer
    }
}

fn explosion_source_loot_facts(
    entities: &EntityStore,
    entity: &Entity,
    source: &ExplosionSourceEntity,
) -> Option<LootEntityFacts> {
    if entity.id() != source.entity_id
        || entity.uuid() != source.uuid
        || entity.data().entity_resource_id() != source.entity_resource_id.as_str()
        || matches!(entity.data(), EntityData::Player)
    {
        return None;
    }
    let entity_type = bbb_protocol::entity_types::vanilla_entity_type(entity.entity_type_id())?;
    let nbt = crate::entity::save::entity_record_tree_without_id(entities, entity).ok()?;
    let position = entity.position();
    Some(LootEntityFacts {
        entity_type: Some(entity_type.resource_id),
        entity_uuid: Some(entity.uuid()),
        living: Some(entity.living().is_some()),
        position: Some(LootEntityPosition {
            x: position.x,
            y: position.y,
            z: position.z,
        }),
        nbt: Some(Arc::new(nbt)),
        ..Default::default()
    })
}

fn stored_explosion_source(
    source: &crate::explosion::ExplosionSourceEntity,
) -> StoredDamageCausingEntity {
    match source.kind {
        crate::explosion::ExplosionSourceEntityKind::Player { instabuild, .. } => {
            StoredDamageCausingEntity::Player {
                uuid: source.uuid,
                entity_id: source.entity_id,
                instabuild,
            }
        }
        crate::explosion::ExplosionSourceEntityKind::Mob => StoredDamageCausingEntity::Mob {
            entity_id: source.entity_id,
        },
        crate::explosion::ExplosionSourceEntityKind::LivingOther
        | crate::explosion::ExplosionSourceEntityKind::Other => StoredDamageCausingEntity::Other {
            entity_id: source.entity_id,
        },
    }
}

fn stored_explosion_damage_rules(
    mob_griefing: bool,
    entity_drops: bool,
    tnt_explodes: bool,
) -> StoredEntityDamageRules {
    StoredEntityDamageRules {
        mob_griefing,
        entity_drops,
        tnt_explodes,
        fall_damage_immune: false,
    }
}

fn explosion_entity_origin(entity: &crate::entity::Entity) -> Vec3d {
    let position = entity.position();
    if matches!(entity.data(), EntityData::Tnt(_)) {
        return position;
    }
    Vec3d {
        y: position.y + f64::from(entity.eye_height()),
        ..position
    }
}

fn ignore_explosion(
    entity: &crate::entity::Entity,
    source_in_water: bool,
    block_affects: bool,
    indirect_source_is_mob: bool,
    mob_griefing: bool,
) -> bool {
    match entity.data() {
        EntityData::Item(_) => !block_affects,
        EntityData::Painting(_)
        | EntityData::ItemFrame(_)
        | EntityData::GlowItemFrame(_)
        | EntityData::LeashKnot(_) => source_in_water || !block_affects,
        EntityData::ArmorStand(_) => {
            // `ArmorStand.isInvisible()` = `Entity.getSharedFlag(5)`.
            let invisible = entity.get_shared_flag(
                bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
            );
            if block_affects {
                invisible
            } else {
                true
            }
        }
        // `Warden.java:367-369`.
        _ if crate::entity::warden::is_digging_or_emerging(
            entity.entity_type_id(),
            entity.pose_id(),
        ) =>
        {
            true
        }
        data if crate::entity::boat::is_boat(data) => {
            crate::entity::boat::ignore_explosion(indirect_source_is_mob, mob_griefing)
        }
        data if crate::entity::minecart::is_minecart(data) => {
            crate::entity::minecart::ignore_explosion(indirect_source_is_mob, mob_griefing)
        }
        _ => false,
    }
}

fn default_explosion_block_particles() -> Vec<bbb_protocol::packets::WeightedExplosionParticle> {
    vec![
        bbb_protocol::packets::WeightedExplosionParticle {
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_POOF_ID,
                raw_options: Vec::new(),
            },
            scaling: 0.5,
            speed: 1.0,
            weight: 1,
        },
        bbb_protocol::packets::WeightedExplosionParticle {
            particle: ParticlePayload {
                particle_type_id: VANILLA_PARTICLE_TYPE_SMOKE_ID,
                raw_options: Vec::new(),
            },
            scaling: 1.0,
            speed: 1.0,
            weight: 1,
        },
    ]
}

#[cfg(test)]
mod vehicle_explosion_tests {
    use super::*;

    #[test]
    fn mob_and_living_other_keep_distinct_stored_damage_rules() {
        let world = ServerWorld::new(crate::world::ServerWorldConfig::default()).unwrap();
        let source = |entity_id, kind| crate::explosion::ExplosionSourceEntity {
            custom_name: None,
            entity_id,
            entity_resource_id: "minecraft:test".to_string(),
            uuid: uuid::Uuid::from_u128(entity_id as u128),
            position: Vec3d::default(),
            kind,
        };

        assert_eq!(
            stored_explosion_source(&source(7, crate::explosion::ExplosionSourceEntityKind::Mob)),
            StoredDamageCausingEntity::Mob { entity_id: 7 }
        );
        assert_eq!(
            stored_explosion_source(&source(
                8,
                crate::explosion::ExplosionSourceEntityKind::LivingOther
            )),
            StoredDamageCausingEntity::Other { entity_id: 8 }
        );

        let mob = source(9, crate::explosion::ExplosionSourceEntityKind::Mob);
        assert!(matches!(
            world.explosion_command_source(&mob),
            Some(crate::commands::CommandDamageSourceEntity::Stored {
                entity_id: 9,
                ref entity_resource_id,
                uuid: Some(uuid),
                position: Some(position),
                is_mob: true,
                is_living_non_player: true,
                ..
            }) if entity_resource_id == "minecraft:test"
                && uuid == mob.uuid
                && position == mob.position
        ));
    }

    #[test]
    fn minecart_container_and_other_vehicle_explosion_ignore_requires_mob_owner_and_disabled_griefing(
    ) {
        let boat = crate::entity::Entity::new(
            1,
            uuid::Uuid::from_u128(1),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            EntityData::Boat(crate::entity::BoatData::new()),
        );

        assert!(ignore_explosion(&boat, false, true, true, false));
        assert!(!ignore_explosion(&boat, false, true, true, true));
        assert!(!ignore_explosion(&boat, false, true, false, false));

        let chest_boat = crate::entity::Entity::new(
            3,
            uuid::Uuid::from_u128(3),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            EntityData::ChestBoat(crate::entity::ChestBoatData::new()),
        );
        assert!(ignore_explosion(&chest_boat, false, true, true, false));
        assert!(!ignore_explosion(&chest_boat, false, true, true, true));
        assert!(!ignore_explosion(&chest_boat, false, true, false, false));

        let minecart = crate::entity::Entity::new(
            2,
            uuid::Uuid::from_u128(2),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
            Vec3d {
                x: 0.0,
                y: 64.0,
                z: 0.0,
            },
            EntityData::Minecart(crate::entity::MinecartData::new()),
        );
        assert!(ignore_explosion(&minecart, false, true, true, false));
        assert!(!ignore_explosion(&minecart, false, true, true, true));
        assert!(!ignore_explosion(&minecart, false, true, false, false));

        for (entity_type_id, data) in [
            (
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
                EntityData::ChestMinecart(crate::entity::MinecartContainerData::new_chest()),
            ),
            (
                bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
                EntityData::HopperMinecart(crate::entity::MinecartContainerData::new_hopper()),
            ),
        ] {
            let container = crate::entity::Entity::new(
                entity_type_id,
                uuid::Uuid::from_u128(entity_type_id as u128),
                entity_type_id,
                Vec3d {
                    x: 0.0,
                    y: 64.0,
                    z: 0.0,
                },
                data,
            );
            assert!(ignore_explosion(&container, false, true, true, false));
            assert!(!ignore_explosion(&container, false, true, true, true));
            assert!(!ignore_explosion(&container, false, true, false, false));
        }
    }

    #[test]
    fn production_explosion_rules_preserve_entity_drops() {
        assert!(!stored_explosion_damage_rules(true, false, true).entity_drops);
        assert!(stored_explosion_damage_rules(true, true, true).entity_drops);
        assert!(!stored_explosion_damage_rules(true, true, false).tnt_explodes);
    }
}

#[cfg(test)]
#[path = "explosion_tests.rs"]
mod sampling_tests;
