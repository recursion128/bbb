//! The elder guardian's mining fatigue pulse over online players.

use super::*;
use crate::entity::ActiveMobEffect;
use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::{send, WorldEvent};
use bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID;
use bbb_protocol::packets::GameEvent;
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

const MINING_FATIGUE: &str = "minecraft:mining_fatigue";

impl ServerWorld {
    /// `ElderGuardian.customServerAiStep` (`ElderGuardian.java:65-78`) and
    /// `MobEffectUtil.addEffectToPlayersAround` (`MobEffectUtil.java:51-73`).
    pub(super) fn apply_elder_guardian_pulse(
        &mut self,
        dimension: &LevelId,
        source: Uuid,
        position: Vec3d,
        silent: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let effect_source = self
            .levels
            .get(dimension)
            .and_then(|level| level.entities.entity_id_for_uuid(source))
            .map(|id| (*dimension, SourceEntity::Stored(id)));
        let board = self.scoreboard.board();
        let source_team = board.players_team(&source.to_string());
        let chosen: Vec<_> = self
            .levels
            .players()
            .filter(|player| {
                player.dimension == *dimension
                    && matches!(player.game_type, GameType::Survival | GameType::Adventure)
                    && position.distance_to_sqr(player.position()) < 50.0 * 50.0
                    && !source_team.is_some_and(|team| {
                        board
                            .players_team(&player.profile.name)
                            .is_some_and(|other| team.name == other.name)
                    })
                    && player.active_effect(MINING_FATIGUE).is_none_or(|effect| {
                        effect.amplifier() < 2
                            || effect.duration() != -1 && effect.duration() <= 1199
                    })
            })
            .map(|player| player.connection_id)
            .collect();
        for connection_id in chosen {
            let effect = ActiveMobEffect::new(MINING_FATIGUE, 6000, 2)
                .expect("vanilla mining fatigue effect");
            self.add_player_mob_effect_from(connection_id, effect, effect_source, events);
            events.push(send(
                connection_id,
                ClientboundPlayPacket::GameEvent(GameEvent {
                    event_id: VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID,
                    param: if silent { 0.0 } else { 1.0 },
                }),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::ConnectionId;
    use crate::entity::ai::mob::ranged::PendingMobSpell;
    use crate::entity::EntityCommand;
    use crate::mailbox::{world_command_mailbox, WorldMailboxLimits};
    use crate::world::ServerWorldConfig;

    fn pulse(world: &mut ServerWorld, source: Uuid, silent: bool) -> Vec<WorldEvent> {
        let dimension = world.levels[ConnectionId(1)].dimension;
        world
            .level_mut_by_id(&dimension)
            .entities
            .queue_command(EntityCommand::MobSpell(
                PendingMobSpell::ElderGuardianPulse {
                    source,
                    position: Vec3d {
                        x: 0.0,
                        y: 80.0,
                        z: 0.0,
                    },
                    silent,
                },
            ));
        let mut events = Vec::new();
        assert_eq!(world.run_entity_commands(&dimension, &mut events), Some(()));
        events
    }

    fn displays(events: &[WorldEvent]) -> Vec<(u64, f32)> {
        let mut displays: Vec<_> = events
            .iter()
            .filter_map(|event| match event {
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::GameEvent(event),
                } if event.event_id == VANILLA_CLIENT_GAME_EVENT_GUARDIAN_ELDER_EFFECT_ID => {
                    Some((connection_id.0, event.param))
                }
                _ => None,
            })
            .collect();
        displays.sort_by_key(|&(id, _)| id);
        displays
    }

    #[test]
    fn elder_guardian_pulse_filters_players_and_refreshes_fatigue_with_display() {
        let mut world = ServerWorld::new(ServerWorldConfig {
            game_type: GameType::Survival,
            ..ServerWorldConfig::default()
        })
        .unwrap();
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        for id in 1..=11 {
            crate::tests::join_loaded(
                &sender,
                &receiver,
                &mut world,
                id,
                &format!("player{id}"),
                id.into(),
            );
        }
        // Join handshakes tick existing players, so stage effects afterwards.
        for id in 1..=11 {
            world.levels[ConnectionId(id)].set_position(Vec3d {
                x: 0.0,
                y: 80.0,
                z: 0.0,
            });
        }
        world.levels[ConnectionId(2)].game_type = GameType::Adventure;
        world.levels[ConnectionId(3)].game_type = GameType::Creative;
        world.levels[ConnectionId(4)].game_type = GameType::Spectator;
        world.levels[ConnectionId(5)].set_position(Vec3d {
            x: 0.0,
            y: 130.0,
            z: 0.0,
        });
        let nether = world.require_level_id(&Java26BuiltinDimension::Nether.into());
        world.levels.move_player(ConnectionId(6), nether);
        let source = Uuid::from_u128(100);
        world.scoreboard.add_team("guardians");
        world
            .scoreboard
            .add_player_to_team(&source.to_string(), "guardians");
        world.scoreboard.add_player_to_team("player7", "guardians");
        for (id, duration, amplifier) in [(8, 6000, 3), (9, 1199, 2), (10, 6000, 1), (11, -1, 2)] {
            world.add_player_mob_effect(
                ConnectionId(id),
                ActiveMobEffect::give(MINING_FATIGUE, duration, amplifier, true).unwrap(),
                &mut Vec::new(),
            );
        }

        let events = pulse(&mut world, source, false);
        assert_eq!(
            displays(&events),
            vec![(1, 1.0), (2, 1.0), (9, 1.0), (10, 1.0)]
        );
        for id in [1, 2, 9, 10] {
            let effect = world.levels[ConnectionId(id)]
                .active_effect(MINING_FATIGUE)
                .unwrap();
            assert_eq!((effect.amplifier(), effect.duration()), (2, 6000));
            assert!(events.iter().any(|event| matches!(event, WorldEvent::Send { connection_id, packet: ClientboundPlayPacket::UpdateMobEffect(_), } if *connection_id == ConnectionId(id))));
        }
        for id in [3, 4, 5, 6, 7] {
            assert!(world.levels[ConnectionId(id)]
                .active_effect(MINING_FATIGUE)
                .is_none());
        }
        assert_eq!(
            world.levels[ConnectionId(8)]
                .active_effect(MINING_FATIGUE)
                .unwrap()
                .duration(),
            6000
        );
        assert_eq!(
            world.levels[ConnectionId(11)]
                .active_effect(MINING_FATIGUE)
                .unwrap()
                .duration(),
            -1
        );

        world.remove_player_mob_effect(ConnectionId(1), MINING_FATIGUE, &mut Vec::new());
        world.add_player_mob_effect(
            ConnectionId(1),
            ActiveMobEffect::new(MINING_FATIGUE, 1199, 3).unwrap(),
            &mut Vec::new(),
        );
        let events = pulse(&mut world, Uuid::from_u128(101), true);
        assert_eq!(displays(&events), vec![(1, 0.0), (7, 0.0)]);
        let effect = world.levels[ConnectionId(1)]
            .active_effect(MINING_FATIGUE)
            .unwrap();
        assert_eq!((effect.amplifier(), effect.duration()), (3, 1199));
    }
}
