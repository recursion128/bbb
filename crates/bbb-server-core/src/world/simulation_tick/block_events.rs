//! World block-event execution and publication.

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    pub(super) fn run_block_events(&mut self, dimension: &LevelId, events: &mut Vec<WorldEvent>) {
        if self.level_by_id(dimension).block_events.is_empty() {
            return;
        }
        let players = self.nearby_players_by_key(dimension);
        let mut leftover = PendingEntitySpawns::default();
        let mut broadcasts: Vec<BlockEventBroadcast> = Vec::new();
        self.with_level_ctx(dimension, &players, &mut leftover, |level, rest| {
            level
                .with_structure_entities(rest.entities, None)
                .run_block_events(&mut broadcasts);
        });
        // `BellBlockEntity.triggerEvent` (`BellBlockEntity.java:43-50`).
        for broadcast in &broadcasts {
            if let BlockEventBroadcast::Event(event) = broadcast {
                if event.block_type_id == bbb_protocol::block_types::VANILLA_BLOCK_BELL_ID
                    && event.param_a == crate::block_entity::bell::EVENT_BELL_RING
                {
                    self.wake_bell_listeners(dimension, event.pos);
                }
            }
        }
        self.post_pending_game_events(dimension);
        self.apply_pending_spawns(dimension, leftover);
        self.broadcast_block_events(dimension, &broadcasts, events);
    }

    pub(super) fn broadcast_block_events(
        &self,
        dimension: &LevelId,
        broadcasts: &[BlockEventBroadcast],
        events: &mut Vec<WorldEvent>,
    ) {
        for broadcast in broadcasts {
            match broadcast {
                BlockEventBroadcast::Sound(sound) => {
                    self.broadcast_level_sound_by_key(dimension, sound, None, events)
                }
                BlockEventBroadcast::Event(event) => {
                    let packet = ClientboundPlayPacket::BlockEvent(event.packet());
                    let mut recipients = Vec::new();
                    for (&connection_id, player) in self.levels.player_entries() {
                        if &player.dimension == dimension
                            && event.reaches(
                                true,
                                player.position().x,
                                player.position().y,
                                player.position().z,
                            )
                        {
                            recipients.push(connection_id);
                        }
                    }
                    crate::world::player_packets::broadcast_to(recipients, packet, events);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_events::BlockEventData;
    use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
    use crate::tests::{console_command, join_loaded, mailbox, place_player};
    use bbb_protocol::block_types::VANILLA_BLOCK_NOTE_BLOCK_ID;

    const POS: BlockPos = BlockPos { x: 1, y: 64, z: 1 };

    fn event(pos: BlockPos) -> BlockEventData {
        BlockEventData::new(pos, VANILLA_BLOCK_NOTE_BLOCK_ID, 0, 0)
    }

    fn note_state(instrument: &str, note: i32) -> i32 {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            "minecraft:note_block",
            &std::collections::BTreeMap::from([
                ("instrument".to_string(), instrument.to_string()),
                ("note".to_string(), note.to_string()),
                ("powered".to_string(), "false".to_string()),
            ]),
        )
        .unwrap()
    }

    fn observed_events(events: &[WorldEvent]) -> Vec<bbb_protocol::packets::BlockEvent> {
        events
            .iter()
            .filter_map(|output| match output {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::BlockEvent(packet),
                    ..
                } => Some(*packet),
                _ => None,
            })
            .collect()
    }

    fn observer() -> NativeLevel {
        let mut level = NativeLevel::new();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut level.world, 1, "listener", 1);
        level
    }

    #[test]
    fn block_events_drop_a_replaced_block_without_broadcast() {
        let mut level = observer();
        level.set(POS, state("minecraft:note_block"));
        level
            .world
            .level_mut(DIMENSION)
            .block_events
            .block_event(event(POS));
        level.set(POS, state("minecraft:chest"));

        let output = level.world.tick(Vec::new());

        assert!(observed_events(&output.events).is_empty());
        assert!(level.world.level(DIMENSION).block_events.is_empty());
    }

    #[test]
    fn block_events_play_the_current_note_after_a_same_block_state_change() {
        let mut level = observer();
        let note = state("minecraft:note_block");
        level.set(POS, note);
        level
            .world
            .level_mut(DIMENSION)
            .block_events
            .block_event(event(POS));
        level.set(POS, note_state("harp", 8));

        let output = level.world.tick(Vec::new());

        assert_eq!(observed_events(&output.events), vec![event(POS).packet()]);
        let sound = output
            .events
            .iter()
            .find_map(|output| match output {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::Sound(sound),
                    ..
                } => Some(sound),
                _ => None,
            })
            .unwrap();
        assert_eq!(sound.pitch, 2.0_f64.powf(-4.0 / 12.0) as f32);
    }

    #[test]
    fn block_events_do_not_broadcast_a_custom_head_without_a_skull_sound() {
        let mut level = observer();
        let note = note_state("custom_head", 0);
        level.set(POS, note);
        level
            .world
            .level_mut(DIMENSION)
            .block_events
            .block_event(event(POS));

        let output = level.world.tick(Vec::new());

        assert!(observed_events(&output.events).is_empty());
        assert!(level.world.level(DIMENSION).block_events.is_empty());
    }

    #[test]
    fn block_events_wait_for_a_player_to_activate_the_column() {
        let mut level = NativeLevel::new();
        let (sender, receiver) = mailbox();
        join_loaded(&sender, &receiver, &mut level.world, 1, "listener", 1);
        let far = BlockPos {
            x: 320,
            y: 64,
            z: 320,
        };
        level.set(far, state("minecraft:note_block"));
        level
            .world
            .level_mut(DIMENSION)
            .block_events
            .block_event(event(far));

        let output = level.world.tick(Vec::new());
        assert!(observed_events(&output.events).is_empty());
        assert!(level
            .world
            .level(DIMENSION)
            .block_events
            .contains(event(far)));

        let moved = console_command(
            &sender,
            &receiver,
            &mut level.world,
            1,
            "tp listener 320.5 65 320.5",
        );
        let mut heard = observed_events(&moved.events);
        let teleport = moved
            .events
            .iter()
            .find_map(|output| match output {
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                    ..
                } => Some(update.id),
                _ => None,
            })
            .unwrap();
        sender
            .try_send(crate::world::WorldCommand::Play {
                connection_id: crate::connection::ConnectionId(1),
                packet: bbb_protocol::packets::PlayServerbound::AcceptTeleportation {
                    id: teleport,
                },
            })
            .unwrap();
        heard.extend(observed_events(&level.world.tick(receiver.drain()).events));
        heard.extend(observed_events(
            &place_player(
                &sender,
                &receiver,
                &mut level.world,
                1,
                Vec3d {
                    x: 320.5,
                    y: 65.0,
                    z: 320.5,
                },
                None,
            )
            .events,
        ));
        for _ in 0..8 {
            if !level
                .world
                .level(DIMENSION)
                .block_events
                .contains(event(far))
            {
                break;
            }
            heard.extend(observed_events(&level.world.tick(Vec::new()).events));
        }
        assert_eq!(heard, vec![event(far).packet()]);
        assert!(!level
            .world
            .level(DIMENSION)
            .block_events
            .contains(event(far)));
    }
}
