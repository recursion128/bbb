use super::test_fixture::CommandTestWorld;
use bbb_protocol::packets::BlockPos;
use bbb_server_types::save::Java26BuiltinDimension;

impl CommandTestWorld {
    pub(crate) fn with_execute_chunk(
        mut self,
        dimension: Java26BuiltinDimension,
        chunk: (i32, i32),
    ) -> Self {
        self.world
            .ensure_entity_transfer_full_chunk_for_test(dimension, chunk);
        self
    }
    pub(crate) fn execute_block(
        &self,
        dimension: Java26BuiltinDimension,
        pos: BlockPos,
    ) -> Option<i32> {
        self.world
            .level(dimension)
            .chunks
            .get_block_state(pos.x, pos.y, pos.z)
    }
    pub(crate) fn execute_player_pose(&self, name: &str) -> ([f64; 3], [f32; 2]) {
        let player = self
            .world
            .levels
            .players()
            .find(|player| player.profile.name == name)
            .unwrap();
        (
            [
                player.position().x,
                player.position().y,
                player.position().z,
            ],
            [player.rotation().y_rot, player.rotation().x_rot],
        )
    }
    pub(crate) fn with_scoreboard_setup_for_execute(mut self) -> Self {
        self.setup("scoreboard objectives add test dummy");
        self
    }
}
