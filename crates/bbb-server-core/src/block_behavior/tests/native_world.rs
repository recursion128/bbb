use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::{Java26ChunkContext, Java26ChunkData, RegionCompression};
use bbb_server_types::chunk_ticks::Java26SavedTick;
use bbb_server_types::save::Java26BuiltinDimension;

use crate::mailbox::{WorldCommandReceiver, WorldCommandSender};
use crate::world::simulation_level::LevelCtx;
use crate::world::{ServerWorld, ServerWorldConfig};

pub(super) struct NativeWorld {
    pub(super) world: ServerWorld,
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
}

impl NativeWorld {
    pub(super) fn new() -> Self {
        let (sender, receiver) = crate::tests::mailbox();
        Self {
            world: ServerWorld::new(ServerWorldConfig {
                datapacks: Some(crate::registry_sync::builtin_datapack_startup_shared()),
                ..Default::default()
            })
            .unwrap(),
            sender,
            receiver,
        }
    }

    pub(super) fn ticking(pos: BlockPos) -> Self {
        use crate::admin::{AdminEnvelope, AdminRequest, AdminScope};
        use crate::world::{WorldCommand, WorldEvent};

        let mut fixture = Self::new();
        fixture
            .sender
            .try_send(WorldCommand::Admin {
                envelope: AdminEnvelope {
                    operation_id: 1,
                    request_id: "forceload".to_string(),
                    principal: "fixture".to_string(),
                    scope: AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 0,
                },
                request: AdminRequest::ExecuteCommand {
                    command: format!("forceload add {} {}", pos.x, pos.z),
                },
            })
            .unwrap();
        let result = fixture.world.tick(fixture.receiver.drain());
        assert!(result.events.iter().any(|event| {
            matches!(event, WorldEvent::Admin(outcome) if outcome.result.is_ok())
        }));
        fixture
    }

    pub(super) fn set(&mut self, pos: BlockPos, state: i32) {
        // Build the scene before the test explicitly dispatches its shape change.
        self.world
            .set_block(Java26BuiltinDimension::Overworld, pos, state, 2 | 16);
        assert_eq!(self.state(pos), state);
    }

    pub(super) fn state(&mut self, pos: BlockPos) -> i32 {
        self.world
            .block_state(Java26BuiltinDimension::Overworld, pos)
            .unwrap()
    }

    pub(super) fn with_level<R>(&mut self, body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R) -> R {
        self.world
            .with_level_for_test(Java26BuiltinDimension::Overworld, body)
    }

    pub(super) fn neighbor_changed(&mut self, pos: BlockPos, source_block_type_id: i32) {
        self.with_level(|level| level.neighbor_changed(pos, source_block_type_id));
    }

    pub(super) fn run_tick(&mut self) {
        self.world.tick(Vec::new());
    }

    pub(super) fn comparator_output(&mut self, pos: BlockPos) -> i32 {
        self.with_level(|level| level.comparator_block_entity_output_signal(pos).unwrap())
    }

    pub(super) fn shape_update(
        &mut self,
        pos: BlockPos,
        direction: bbb_protocol::packets::Direction,
    ) {
        let neighbor_pos = bbb_server_types::directions::relative(pos, direction);
        let neighbor_state = self.state(neighbor_pos);
        self.with_level(|level| {
            level.neighbor_shape_changed(
                direction,
                pos,
                neighbor_pos,
                neighbor_state,
                crate::fluid::SET_BLOCK_UPDATE_ALL,
                crate::neighbor_updater::UPDATE_LIMIT,
            )
        });
    }

    /// Random-ticks `pos` until some block changes, at most 4096 times: a
    /// growth roll as rare as 1 in 300 then misses with odds under 1e-5.
    pub(super) fn random_tick(&mut self, pos: BlockPos, seed: i64) {
        let state = self.state(pos);
        self.with_level(|level| {
            level.level_random().reseed(seed);
            let before = level.save_dirty_epoch_for_test();
            for _ in 0..4096 {
                crate::block_behavior::random_tick_block(level, pos, state);
                if level.save_dirty_epoch_for_test() != before {
                    break;
                }
            }
        });
    }

    pub(super) fn illuminate(&mut self, pos: BlockPos, brightness: i32) {
        use super::offset;
        use crate::block_behavior::LightLayer;

        let distance = if brightness == 0 { 1 } else { 15 - brightness };
        for x in -2..=distance + 1 {
            for y in -2..=1 {
                for z in -2_i32..=2 {
                    if x == -2 || x == distance + 1 || y == -2 || y == 1 || z.abs() == 2 {
                        let boundary = offset(pos, x, y, z);
                        self.set(boundary, super::stone());
                    }
                }
            }
        }
        for x in 1..=distance {
            self.set(offset(pos, x, 0, 0), super::air());
        }
        if brightness > 0 {
            self.set(
                offset(pos, distance, 0, 0),
                super::default_state("minecraft:glowstone"),
            );
        }
        assert_eq!(
            self.with_level(|level| level.brightness(LightLayer::Block, pos)),
            brightness
        );
        assert_eq!(
            self.with_level(|level| level.brightness(LightLayer::Sky, pos)),
            0
        );
    }

    pub(super) fn command(&mut self, command: &str) {
        use crate::admin::{AdminEnvelope, AdminRequest, AdminScope};
        use crate::world::{WorldCommand, WorldEvent};

        self.sender
            .try_send(WorldCommand::Admin {
                envelope: AdminEnvelope {
                    operation_id: 2,
                    request_id: uuid::Uuid::new_v4().to_string(),
                    principal: "fixture".into(),
                    scope: AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 0,
                },
                request: AdminRequest::ExecuteCommand {
                    command: command.to_string(),
                },
            })
            .unwrap();
        let result = self.world.tick(self.receiver.drain());
        assert!(result
            .events
            .iter()
            .any(|event| matches!(event, WorldEvent::Admin(outcome) if outcome.result.is_ok())));
    }

    pub(super) fn join_listener(&mut self, position: bbb_protocol::packets::Vec3d) {
        crate::tests::join_loaded(
            &self.sender,
            &self.receiver,
            &mut self.world,
            1,
            "listener",
            1,
        );
        self.world
            .set_player_position_for_test(crate::connection::ConnectionId(1), position);
    }

    pub(super) fn tick_block(&mut self, pos: BlockPos) {
        let state = self.state(pos);
        self.with_level(|level| crate::block_behavior::tick_block(level, pos, state));
    }

    pub(super) fn entities(&self) -> Vec<&crate::entity::Entity> {
        self.world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter_map(|id| {
                self.world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)
            })
            .collect()
    }

    pub(super) fn items(&self) -> Vec<(String, i32)> {
        self.world
            .stored_items_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .map(|(_, _, id, count, _, _)| (id, count))
            .collect()
    }

    pub(super) fn falling_blocks(&self) -> Vec<&crate::entity::FallingBlockData> {
        self.world
            .stored_entity_ids_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .filter_map(|id| {
                let entity = self
                    .world
                    .stored_entity_for_test(Java26BuiltinDimension::Overworld, id)?;
                match entity.data() {
                    crate::entity::EntityData::FallingBlock(data) => Some(data),
                    _ => None,
                }
            })
            .collect()
    }

    pub(super) fn level_events(&mut self) -> Vec<crate::level_event::PendingLevelEvent> {
        self.world
            .drain_level_events_for_test(Java26BuiltinDimension::Overworld)
    }

    pub(super) fn sounds(&mut self) -> Vec<crate::level_event::PendingBlockSound> {
        self.world
            .drain_block_sounds_for_test(Java26BuiltinDimension::Overworld)
    }

    pub(super) fn events(&self) -> Vec<crate::level_event::RecordedGameEvent> {
        self.world
            .positioned_game_events_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .map(|event| event.recorded)
            .collect()
    }

    pub(super) fn saved_fluid_ticks(&mut self) -> Vec<Java26SavedTick> {
        self.saved_ticks().1
    }

    pub(super) fn saved_block_ticks(&mut self) -> Vec<Java26SavedTick> {
        self.saved_ticks().0
    }

    /// The block name the saved column document holds at `pos`.
    pub(super) fn saved_block_name(&mut self, pos: BlockPos) -> String {
        let saved = self
            .world
            .save_request(0, 0, RegionCompression::Zlib)
            .unwrap();
        let write = saved
            .chunk_writes
            .into_iter()
            .find(|write| (write.chunk_x, write.chunk_z) == (pos.x >> 4, pos.z >> 4))
            .expect("the edited column is written");
        let chunk = Java26ChunkData::parse(
            write.document,
            Java26ChunkContext {
                expected_x: write.chunk_x,
                expected_z: write.chunk_z,
                min_section_y: -4,
                section_count: 24,
            },
        )
        .unwrap();
        let section = chunk
            .sections()
            .iter()
            .find(|section| i32::from(section.y) == pos.y >> 4)
            .expect("the section holding pos");
        section
            .block_states
            .as_ref()
            .unwrap()
            .value_at(
                (pos.x & 15) as usize,
                (pos.y & 15) as usize,
                (pos.z & 15) as usize,
            )
            .unwrap()
            .name
            .clone()
    }

    fn saved_ticks(&mut self) -> (Vec<Java26SavedTick>, Vec<Java26SavedTick>) {
        let saved = self
            .world
            .save_request(0, 0, RegionCompression::Zlib)
            .unwrap();
        let mut blocks = Vec::new();
        let mut fluids = Vec::new();
        for chunk in saved.chunk_writes {
            let chunk = Java26ChunkData::parse(
                chunk.document,
                Java26ChunkContext {
                    expected_x: chunk.chunk_x,
                    expected_z: chunk.chunk_z,
                    min_section_y: -4,
                    section_count: 24,
                },
            )
            .unwrap();
            blocks.extend_from_slice(chunk.block_ticks());
            fluids.extend_from_slice(chunk.fluid_ticks());
        }
        (blocks, fluids)
    }
}
