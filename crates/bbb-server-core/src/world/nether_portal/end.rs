use super::*;
use crate::entity::EntityData;
use crate::world::LevelId;

// ServerLevel.java:186.
const END_SPAWN_POINT: BlockPos = BlockPos {
    x: 100,
    y: 50,
    z: 0,
};

impl ServerWorld {
    // EndPortalBlock.java:74-110; Entity.java:1324-1329.
    pub(super) fn transfer_stored_entity_through_end_portal(
        &mut self,
        source: &LevelId,
        entity_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> SaveResult<bool> {
        let from_end = source.builtin() == Some(Java26BuiltinDimension::End);
        let target = if from_end {
            self.cold.default_spawn.dimension
        } else {
            Java26BuiltinDimension::End.into()
        };
        if !self.has_level_id(&target) {
            return Ok(false);
        }
        // Entity.canTeleport: first-time credits require an unmounted End exit.
        if from_end && target.builtin() == Some(Java26BuiltinDimension::Overworld) {
            let entities = &self.level_by_id(source).entities;
            // ThrownEnderpearl.java:206-209.
            if let Some(EntityData::EnderPearl(data)) = entities.get(entity_id).map(|e| e.data()) {
                if data
                    .projectile()
                    .owner()
                    .and_then(|owner| self.connection_of(owner))
                    .is_some_and(|connection| !self.levels[connection].cold.seen_credits)
                {
                    return Ok(false);
                }
            }
            if entities.passengers(entity_id).iter().any(|id| {
                entities
                    .player(*id)
                    .is_some_and(|player| !player.cold.seen_credits)
            }) {
                return Ok(false);
            }
        }
        let spawn = if from_end {
            block_position(self.cold.default_spawn.position)
        } else {
            END_SPAWN_POINT
        };
        if from_end {
            self.generate_original_mob_columns(&target, &[(spawn.x >> 4, spawn.z >> 4)])?;
        }
        let position = Vec3d {
            x: f64::from(spawn.x) + 0.5,
            y: if from_end {
                f64::from(self.level_by_id(&target).chunks.final_heightmap_first_available(
                    spawn.x >> 4,
                    spawn.z >> 4,
                    bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
                    spawn.x & 15,
                    spawn.z & 15,
                ).ok_or_else(|| invalid_search("End exit heightmap is unavailable"))?)
            } else {
                f64::from(spawn.y)
            },
            z: f64::from(spawn.z) + 0.5,
        };
        let transition = StoredEntityTransition {
            position,
            delta_movement: Vec3d::default(),
            y_rot: if from_end {
                self.cold.default_spawn.rotation.y_rot
            } else {
                90.0
            },
            x_rot: if from_end {
                self.cold.default_spawn.rotation.x_rot
            } else {
                0.0
            },
            relatives_mask: if from_end {
                PORTAL_RELATIVES_MASK
            } else {
                PORTAL_RELATIVES_MASK & !(1 << 3)
            },
        };
        let Some(tree) = super::super::entity_transfer::resolved_tree(
            &self.level_by_id(source).entities,
            entity_id,
            transition,
            true,
        )
        .map_err(|error| invalid_search(format!("End tree: {error}")))?
        else {
            return Ok(false);
        };
        // Each passenger's post-transition places its own ticket before the root's.
        let arrivals = tree
            .into_iter()
            .map(|(_, position, _)| block_position(position))
            .collect::<Vec<_>>();
        let ticket = self.prepare_portal_tickets_by_key(&target, &arrivals, &[])?;
        let ticket_commit = self.publish_portal_destination(ticket);
        if !from_end {
            self.create_end_spawn_platform(
                target,
                BlockPos {
                    y: spawn.y - 1,
                    ..spawn
                },
            );
        }
        self.level_mut_by_id(&target).chunks.flush_live_relights()?;
        if target.builtin() == Some(Java26BuiltinDimension::Nether)
            && !self.game_rule_bool(RULE_ALLOW_ENTERING_NETHER_USING_PORTALS, true)
        {
            return Ok(false);
        }
        if self
            .teleport_entity_subtree(*source, target, entity_id, transition, true, events)
            .map_err(|error| invalid_search(format!("End transfer: {error}")))?
            .is_none()
        {
            return Ok(false);
        }
        self.commit_portal_ticket_state(ticket_commit);
        Ok(true)
    }
}

pub(in crate::world) struct PlayerEndPortalDestination {
    pub(in crate::world) transition: super::super::TeleportTransition<LevelId>,
    /// `PLAY_PORTAL_SOUND.then(PLACE_PORTAL_TICKET)`; leaving the End is
    /// `DO_NOTHING`.
    pub(in crate::world) ticket: Option<super::super::portal_tickets::PreparedPortalTicket>,
    pub(in crate::world) missing_respawn_block: bool,
}

impl ServerWorld {
    // EndPortalBlock.java:89-122 for a ServerPlayer: into the End it lands one
    // block lower than other entities; out of it, it is the player's own
    // respawn (`findRespawnPositionAndUseSpawnBlock(false, DO_NOTHING)`).
    pub(in crate::world) fn player_end_portal_destination(
        &mut self,
        source: &LevelId,
        respawn: Option<&super::super::PlayerRespawnConfig>,
    ) -> SaveResult<Option<PlayerEndPortalDestination>> {
        if source.builtin() == Some(Java26BuiltinDimension::End) {
            let (spawn, missing_respawn_block) = self
                .resolve_sleep_respawn(respawn, false)
                .map_err(|error| invalid_search(format!("End exit respawn: {error}")))?;
            if !self.has_level_id(&spawn.dimension) {
                return Ok(None);
            }
            self.install_full_chunks_by_key(
                &spawn.dimension,
                &[super::super::player_chunk(spawn.position)],
            )?;
            return Ok(Some(PlayerEndPortalDestination {
                transition: super::super::TeleportTransition {
                    new_level: spawn.dimension,
                    position: spawn.position,
                    y_rot: spawn.rotation.y_rot,
                    x_rot: spawn.rotation.x_rot,
                    relatives_mask: 0,
                },
                ticket: None,
                missing_respawn_block,
            }));
        }
        let target = LevelId::from(Java26BuiltinDimension::End);
        if !self.has_level_id(&target) {
            return Ok(None);
        }
        let position = Vec3d {
            x: f64::from(END_SPAWN_POINT.x) + 0.5,
            y: f64::from(END_SPAWN_POINT.y - 1),
            z: f64::from(END_SPAWN_POINT.z) + 0.5,
        };
        let ticket = self.prepare_portal_ticket_by_key(&target, block_position(position), &[])?;
        let ticket = self.publish_portal_destination(ticket);
        self.create_end_spawn_platform(
            target,
            BlockPos {
                y: END_SPAWN_POINT.y - 1,
                ..END_SPAWN_POINT
            },
        );
        self.level_mut_by_id(&target).chunks.flush_live_relights()?;
        Ok(Some(PlayerEndPortalDestination {
            transition: super::super::TeleportTransition {
                new_level: target,
                position,
                // `Direction.WEST.toYRot()`.
                y_rot: 90.0,
                x_rot: 0.0,
                relatives_mask: PORTAL_RELATIVES_MASK & !(1 << 3),
            },
            ticket: Some(ticket),
            missing_respawn_block: false,
        }))
    }
}
