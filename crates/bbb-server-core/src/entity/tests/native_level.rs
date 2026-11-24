use std::sync::OnceLock;

use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::{Java26SavedChunkTicket, Java26WorldCandidate};
use bbb_server_types::interaction::WorldBorder;
use bbb_server_types::save::Java26BuiltinDimension;

use crate::registry_tags::RegistryTagSnapshot;
use crate::world::simulation_level::LevelCtx;
use crate::world::{ServerWorld, ServerWorldConfig};

pub(crate) const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

pub(crate) struct NativeLevel {
    pub(crate) world: ServerWorld,
}

fn candidate() -> &'static Java26WorldCandidate {
    static CANDIDATE: OnceLock<Java26WorldCandidate> = OnceLock::new();
    CANDIDATE.get_or_init(|| {
        let root = std::env::temp_dir().join(format!("bbb-native-entity-{}", std::process::id()));
        let storage = crate::tests::created_fixture_save(&root, "Native entity tests");
        let mut candidate = storage.load_candidate().unwrap();
        candidate.chunk_tickets.insert(
            DIMENSION.into(),
            [(0, 0), (4, 4), (5, 4)]
                .into_iter()
                .map(|(chunk_x, chunk_z)| Java26SavedChunkTicket {
                    chunk_x,
                    chunk_z,
                    level: 31,
                    ticks_left: None,
                })
                .collect(),
        );
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
        candidate
    })
}

impl NativeLevel {
    pub(crate) fn new() -> Self {
        Self::with_datapacks(crate::registry_sync::builtin_datapack_startup_shared())
    }

    pub(crate) fn with_datapacks(
        datapacks: std::sync::Arc<bbb_server_datapack::startup::DatapackStartup>,
    ) -> Self {
        Self {
            world: ServerWorld::restore(
                ServerWorldConfig {
                    datapacks: Some(datapacks),
                    ..crate::tests::creative_fixture_config()
                },
                candidate(),
            )
            .unwrap(),
        }
    }

    pub(crate) fn with_level<R>(&mut self, body: impl FnOnce(&mut LevelCtx<'_, '_>) -> R) -> R {
        self.world.with_level_for_test(DIMENSION, body)
    }

    pub(crate) fn set_scoreboard_teams(&mut self, teams: crate::entity::ScoreboardTeams) {
        let mut board = crate::world::scoreboard::Scoreboard::default();
        board.teams = teams;
        *self.world.scoreboard_mut() =
            crate::world::server_scoreboard::ServerScoreboard::restore(&board.pack());
    }

    pub(crate) fn set(&mut self, pos: BlockPos, state: i32) {
        assert!(self.world.set_block(
            DIMENSION,
            pos,
            state,
            crate::block_behavior::SET_BLOCK_UPDATE_CLIENTS,
        ));
    }

    pub(crate) fn install_tags(&mut self, tags: RegistryTagSnapshot) {
        self.world
            .install_level_registry_tags(std::sync::Arc::new(tags));
    }

    pub(crate) fn set_border(&mut self, border: WorldBorder) {
        self.world.set_world_border_for_test(DIMENSION, border);
    }
}

pub(crate) fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}
