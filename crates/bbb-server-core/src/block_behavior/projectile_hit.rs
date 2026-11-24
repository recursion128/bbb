//! `BlockBehaviour.onProjectileHit`, which `Projectile.onHitBlock`
//! (`Projectile.java:312-315`) runs for every projectile that hits a block.

use bbb_protocol::block_types::{
    VANILLA_BLOCK_AMETHYST_BLOCK_ID, VANILLA_BLOCK_AMETHYST_CLUSTER_ID, VANILLA_BLOCK_BELL_ID,
    VANILLA_BLOCK_BIG_DRIPLEAF_ID, VANILLA_BLOCK_BUDDING_AMETHYST_ID, VANILLA_BLOCK_CAMPFIRE_ID,
    VANILLA_BLOCK_CHORUS_FLOWER_ID, VANILLA_BLOCK_DECORATED_POT_ID,
    VANILLA_BLOCK_LARGE_AMETHYST_BUD_ID, VANILLA_BLOCK_MEDIUM_AMETHYST_BUD_ID,
    VANILLA_BLOCK_POINTED_DRIPSTONE_ID, VANILLA_BLOCK_SMALL_AMETHYST_BUD_ID,
    VANILLA_BLOCK_SOUL_CAMPFIRE_ID, VANILLA_BLOCK_TARGET_ID, VANILLA_BLOCK_TNT_ID,
};
use bbb_protocol::packets::{BlockPos, Direction, SoundSource, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use super::{block_type_of, footing, LevelCtx};
use crate::entity::projectile_action::PendingProjectileAction;
use crate::entity::store::{EntityStore, PendingEntitySound, PendingPlayerStat};
use crate::entity::{Entity, EntityCommand, EntityData};
use crate::level_event::VanillaGameEvent;

/// Where a projectile hit a block: `BlockHitResult`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ProjectileBlockHit {
    pub location: Vec3d,
    pub pos: BlockPos,
    pub direction: Direction,
}

/// `Projectile.getOwner()` as the overrides read it.
#[derive(Debug, Clone, Copy)]
enum Owner {
    None,
    Player {
        uuid: Uuid,
        entity_id: i32,
    },
    Stored {
        uuid: Uuid,
        entity_id: i32,
        living: bool,
    },
}

impl Owner {
    fn of(projectile: &Entity, store: Option<&EntityStore>, level: &LevelCtx<'_, '_>) -> Self {
        let Some(uuid) = projectile.data().projectile_owner() else {
            return Self::None;
        };
        if let Some(player) = level.player(uuid) {
            return Self::Player {
                uuid,
                entity_id: player.entity_id,
            };
        }
        store
            .and_then(|store| store.entity_id_for_uuid(uuid).and_then(|id| store.get(id)))
            .map_or(Self::None, |owner| Self::Stored {
                uuid,
                entity_id: owner.id(),
                living: owner.living().is_some(),
            })
    }

    const fn player(self) -> Option<Uuid> {
        match self {
            Self::Player { uuid, .. } => Some(uuid),
            _ => None,
        }
    }

    /// `instanceof LivingEntity`.
    const fn living(self) -> Option<(Uuid, i32)> {
        match self {
            Self::Player { uuid, entity_id } => Some((uuid, entity_id)),
            Self::Stored {
                uuid,
                entity_id,
                living: true,
            } => Some((uuid, entity_id)),
            _ => None,
        }
    }
}

/// `Projectile.mayInteract` (`Projectile.java:361-364`).
fn may_interact(owner: Owner, level: &LevelCtx<'_, '_>, pos: BlockPos) -> bool {
    match owner {
        Owner::None => true,
        Owner::Player { uuid, .. } => level.player_may_interact(uuid, pos),
        Owner::Stored { .. } => level.mob_griefing(),
    }
}

pub(crate) fn entity_may_interact(
    entity: &Entity,
    store: Option<&EntityStore>,
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
) -> bool {
    may_interact(Owner::of(entity, store, level), level, pos)
}

/// `Projectile.mayBreak` (`Projectile.java:366-368`).
fn may_break(projectile: &Entity, level: &LevelCtx<'_, '_>) -> bool {
    level.projectiles_can_break_blocks()
        && level.registry_tag_membership(
            "minecraft:entity_type",
            "minecraft:impact_projectiles",
            projectile.data().entity_resource_id(),
        )
}

pub(crate) fn on_projectile_hit(
    level: &mut LevelCtx<'_, '_>,
    store: &mut EntityStore,
    projectile: &Entity,
    hit: ProjectileBlockHit,
) {
    let ProjectileBlockHit {
        location,
        pos,
        direction,
    } = hit;
    let state = level.block_state(pos);
    let Some(block) = block_type_of(state) else {
        return;
    };
    let owner = Owner::of(projectile, Some(store), level);
    match block {
        VANILLA_BLOCK_BELL_ID => {
            let ringer = match owner {
                Owner::Player { entity_id, .. } => Some(entity_id),
                _ => None,
            };
            let rang = super::bell::on_projectile_hit(
                level,
                pos,
                state,
                direction,
                location.y - f64::from(pos.y),
                ringer,
            );
            if let (true, Some(player)) = (rang, owner.player()) {
                award_stat(store, player, "minecraft:bell_ring");
            }
        }
        VANILLA_BLOCK_TARGET_ID => {
            let arrow = matches!(
                projectile.data(),
                EntityData::Arrow(_) | EntityData::SpectralArrow(_) | EntityData::Trident(_)
            );
            let signal =
                super::target::on_projectile_hit(level, pos, state, location, direction, arrow);
            if let Some(player) = owner.player() {
                award_stat(store, player, "minecraft:target_hit");
                store.queue_command(EntityCommand::Projectile(
                    PendingProjectileAction::TargetHit {
                        player,
                        projectile: Box::new(projectile.clone()),
                        signal,
                    },
                ));
            }
        }
        // `TntBlock.onProjectileHit` (`TntBlock.java:133-143`) over `prime`
        // (`:88-98`).
        VANILLA_BLOCK_TNT_ID => {
            if projectile.is_on_fire() && may_interact(owner, level, pos) && level.tnt_explodes() {
                let position = Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y),
                    z: f64::from(pos.z) + 0.5,
                };
                let source = owner.living();
                if crate::entity::spawn_tnt(
                    store,
                    level.level_random(),
                    position,
                    source.map(|(uuid, _)| uuid),
                )
                .is_none()
                {
                    return;
                }
                store.play_entity_sound(PendingEntitySound {
                    position,
                    resource_id: "minecraft:entity.tnt.primed".to_string(),
                    source: SoundSource::Blocks,
                    volume: 1.0,
                    pitch: 1.0,
                });
                level.game_event_from_entity(
                    VanillaGameEvent::PRIME_FUSE,
                    pos,
                    source.map(|(_, id)| id),
                    None,
                );
                level.remove_block(pos);
            }
        }
        VANILLA_BLOCK_CAMPFIRE_ID | VANILLA_BLOCK_SOUL_CAMPFIRE_ID => {
            if projectile.is_on_fire() && may_interact(owner, level, pos) {
                super::campfire::light_by_projectile(level, pos, state);
            }
        }
        // `ChorusFlowerBlock.onProjectileHit` (`ChorusFlowerBlock.java:257-262`).
        VANILLA_BLOCK_CHORUS_FLOWER_ID => {
            if may_interact(owner, level, pos) && may_break(projectile, level) {
                level.destroy_block(pos, true);
            }
        }
        // `DecoratedPotBlock.onProjectileHit` (`DecoratedPotBlock.java:217-223`).
        VANILLA_BLOCK_DECORATED_POT_ID => {
            if may_interact(owner, level, pos) && may_break(projectile, level) {
                super::player_break::crack_decorated_pot(level, pos, state);
                level.destroy_block(pos, true);
            }
        }
        // `AmethystBlock.onProjectileHit` (`AmethystBlock.java:26-31`), which
        // budding amethyst and every cluster and bud inherit.
        VANILLA_BLOCK_AMETHYST_BLOCK_ID
        | VANILLA_BLOCK_BUDDING_AMETHYST_ID
        | VANILLA_BLOCK_AMETHYST_CLUSTER_ID
        | VANILLA_BLOCK_LARGE_AMETHYST_BUD_ID
        | VANILLA_BLOCK_MEDIUM_AMETHYST_BUD_ID
        | VANILLA_BLOCK_SMALL_AMETHYST_BUD_ID => {
            let pitch = 0.5 + level.level_random().next_float() * 1.2;
            level.play_block_sound_with_volume_pitch(
                pos,
                "minecraft:block.amethyst_block.chime",
                1.0,
                pitch,
            );
        }
        VANILLA_BLOCK_BIG_DRIPLEAF_ID => super::big_dripleaf::on_projectile_hit(level, pos, state),
        // `PointedDripstoneBlock.onProjectileHit` (`PointedDripstoneBlock.java:141-152`).
        VANILLA_BLOCK_POINTED_DRIPSTONE_ID => {
            if matches!(projectile.data(), EntityData::Trident(_))
                && projectile.delta_movement().length_sqr().sqrt() > 0.6
                && may_interact(owner, level, pos)
                && may_break(projectile, level)
            {
                level.destroy_block(pos, true);
            }
        }
        _ if (super::candle::is_candle(state) || footing::is_candle_cake(state))
            && projectile.is_on_fire() =>
        {
            super::candle::light_by_projectile(level, pos, state);
        }
        _ => {}
    }
}

fn award_stat(store: &mut EntityStore, player: Uuid, stat: &'static str) {
    store.queue_command(EntityCommand::PlayerStat(PendingPlayerStat {
        player_uuid: player,
        stat,
        amount: 1,
    }));
}
