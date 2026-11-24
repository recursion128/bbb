use super::*;
use crate::world::LevelId;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(in crate::world) struct BlockInteractionPolicy {
    operators: BTreeSet<Uuid>,
    spawn_protection_radius: i32,
    dimension: LevelId,
    spawn_dimension: LevelId,
    spawn_position: BlockPos,
}

impl BlockInteractionPolicy {
    /// DedicatedServer.isUnderSpawnProtection (DedicatedServer.java:548-570).
    pub(in crate::world) fn under_spawn_protection(&self, uuid: Uuid, pos: BlockPos) -> bool {
        if self.dimension != self.spawn_dimension
            || self.operators.is_empty()
            || self.operators.contains(&uuid)
            || self.spawn_protection_radius <= 0
        {
            return false;
        }
        let dx = (i64::from(pos.x) - i64::from(self.spawn_position.x)).abs();
        let dz = (i64::from(pos.z) - i64::from(self.spawn_position.z)).abs();
        dx.max(dz) <= i64::from(self.spawn_protection_radius)
    }

    /// ServerLevel.mayInteract (ServerLevel.java:857-859).
    pub(in crate::world) fn player_may_interact(
        &self,
        uuid: Uuid,
        pos: BlockPos,
        world_border: WorldBorder,
        game_time: i64,
    ) -> bool {
        !self.under_spawn_protection(uuid, pos) && world_border.contains_block_at(pos, game_time)
    }
}

impl ServerWorld {
    pub(in crate::world) fn block_interaction_policy(
        &self,
        dimension: &LevelId,
    ) -> BlockInteractionPolicy {
        let mut spawn_position = block_position(self.cold.default_spawn.position);
        // MinecraftServer.java:1148-1151; Level.java:747-755. Protection only reads X/Z.
        if let Some(level) = self.level_if_present_by_id(&self.cold.default_spawn.dimension) {
            let border = level.world_border;
            if !border.contains_block_at(spawn_position, self.game_time()) {
                spawn_position.x = border.center_x().floor() as i32;
                spawn_position.z = border.center_z().floor() as i32;
            }
        }
        BlockInteractionPolicy {
            operators: self.admin.lists().operators.keys().copied().collect(),
            spawn_protection_radius: self.admin.settings().spawn_protection_radius,
            dimension: *dimension,
            spawn_dimension: self.cold.default_spawn.dimension,
            spawn_position,
        }
    }

    pub(in crate::world) fn player_is_under_spawn_protection(
        &self,
        id: ConnectionId,
        pos: BlockPos,
    ) -> bool {
        let Some(player) = self.levels.player(id) else {
            return false;
        };
        self.block_interaction_policy(&player.dimension)
            .under_spawn_protection(player.profile.uuid, pos)
    }

    pub(in crate::world) fn player_may_interact_block(
        &self,
        id: ConnectionId,
        pos: BlockPos,
    ) -> bool {
        let Some(player) = self.levels.player(id) else {
            return false;
        };
        let Some(level) = self.level_if_present_by_id(&player.dimension) else {
            return false;
        };
        self.block_interaction_policy(&player.dimension)
            .player_may_interact(
                player.profile.uuid,
                pos,
                level.world_border,
                self.game_time(),
            )
    }

    /// `ServerLevel.mayInteract` at the two packet entries
    /// (ServerPlayerGameMode.java:160-170, ServerGamePacketListenerImpl.java:1352-1354):
    /// answers whether the block is refused, telling a protected player why.
    pub(in crate::world) fn refuse_block_interaction(
        &self,
        id: ConnectionId,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        if self.player_is_under_spawn_protection(id, pos) {
            self.send_system_message(
                id,
                bbb_protocol::packets::SystemChat::from_component(
                    crate::commands::spawn_protection_message(pos.x, pos.y, pos.z),
                    true,
                ),
                events,
            );
            return true;
        }
        !self.player_may_interact_block(id, pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admin::{NameAndId, OperatorEntry, StoredUserLists};
    use crate::commands::CommandPermission;

    #[test]
    fn spawn_protection_uses_operator_membership_and_global_spawn_dimension() {
        let operator = NameAndId {
            id: Uuid::from_u128(1),
            name: "Operator".into(),
        };
        let visitor = Uuid::from_u128(2);
        let mut lists = StoredUserLists::default();
        lists.operators.insert(
            operator.id,
            OperatorEntry {
                user: operator.clone(),
                level: CommandPermission::All,
                bypasses_player_limit: false,
            },
        );
        let mut world = ServerWorld::new(ServerWorldConfig {
            admin_lists: lists,
            ..ServerWorldConfig::default()
        })
        .unwrap();
        let overworld = LevelId::from(Java26BuiltinDimension::Overworld);
        let spawn = block_position(world.cold.default_spawn.position);
        let policy = world.block_interaction_policy(&overworld);
        assert!(policy.under_spawn_protection(visitor, spawn));
        assert!(!policy.under_spawn_protection(operator.id, spawn));
        let protected = BlockPos {
            x: spawn.x + 16,
            y: spawn.y + 100,
            z: spawn.z - 16,
        };
        assert!(!policy.player_may_interact(visitor, protected, WorldBorder::DEFAULT, 0));
        assert!(policy.player_may_interact(
            visitor,
            BlockPos {
                x: spawn.x + 17,
                ..spawn
            },
            WorldBorder::DEFAULT,
            0
        ));
        assert!(!policy.player_may_interact(
            operator.id,
            BlockPos {
                x: 30_000_000,
                ..spawn
            },
            WorldBorder::DEFAULT,
            0
        ));

        let nether = LevelId::from(Java26BuiltinDimension::Nether);
        world.cold.default_spawn.dimension = nether;
        assert!(!world
            .block_interaction_policy(&overworld)
            .under_spawn_protection(visitor, spawn));
        assert!(world
            .block_interaction_policy(&nether)
            .under_spawn_protection(visitor, spawn));
        world.level_mut_by_id(&nether).world_border =
            WorldBorder::from_settings(100.75, -100.25, 0.2, 5.0, 5, 15, 20.0, 0, 20.0, 0);
        let adjusted = BlockPos {
            x: 100,
            z: -101,
            ..spawn
        };
        assert!(world
            .block_interaction_policy(&nether)
            .under_spawn_protection(visitor, adjusted));
        assert!(!world
            .block_interaction_policy(&nether)
            .under_spawn_protection(visitor, spawn));
        let (sender, receiver) = crate::tests::mailbox();
        sender
            .try_send(WorldCommand::Admin {
                envelope: crate::admin::AdminEnvelope {
                    operation_id: 1,
                    request_id: "disable-protection".into(),
                    principal: "agent".into(),
                    scope: crate::admin::AdminScope::Manage,
                    deadline_tick: u64::MAX,
                    now: 0,
                },
                request: crate::admin::AdminRequest::SetSetting(
                    crate::admin::AdminSetting::SpawnProtectionRadius(0),
                ),
            })
            .unwrap();
        let tick = world.tick(receiver.drain());
        let result = tick.events.iter().find_map(|event| match event {
            WorldEvent::Admin(outcome) => Some(&outcome.result),
            _ => None,
        });
        assert_eq!(
            result,
            Some(&Ok(crate::admin::AdminResult::Setting(
                crate::admin::AdminSetting::SpawnProtectionRadius(0)
            )))
        );
        assert_eq!(world.cold.config.spawn_protection_radius, 16);
        assert!(!world
            .block_interaction_policy(&nether)
            .under_spawn_protection(visitor, adjusted));
        assert!(policy.under_spawn_protection(visitor, spawn));
    }

    #[test]
    fn live_player_block_permission_allows_an_empty_operator_list() {
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "Visitor", 2);
        let spawn = block_position(world.cold.default_spawn.position);
        assert!(!world.player_is_under_spawn_protection(ConnectionId(1), spawn));
        assert!(world.player_may_interact_block(ConnectionId(1), spawn));
        assert!(!world.player_may_interact_block(
            ConnectionId(1),
            BlockPos {
                x: 30_000_000,
                ..spawn
            }
        ));
    }
}
