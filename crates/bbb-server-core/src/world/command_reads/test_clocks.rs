use super::test_fixture::CommandTestWorld;
use bbb_server_persistence::Java26WorldClockState;

impl CommandTestWorld {
    pub(crate) fn with_clock_ticks(mut self, ticks: &[i64]) -> Self {
        for (index, &total_ticks) in ticks.iter().enumerate() {
            self.world.set_world_clock_state_for_test(
                index,
                Java26WorldClockState {
                    total_ticks,
                    ..Default::default()
                },
            );
        }
        self
    }

    pub(crate) fn with_game_time(mut self, game_time: i64) -> Self {
        self.world.game_time = game_time;
        self
    }

    pub(crate) fn clock_state(&self, index: usize) -> Java26WorldClockState {
        self.world.world_clock_snapshot_for_test().entries()[index].state
    }
}
