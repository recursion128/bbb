use super::test_fixture::CommandTestWorld;
use crate::commands::CommandAttributeTarget;
use bbb_protocol::packets::WaypointIcon;

impl CommandTestWorld {
    pub(crate) fn with_player_waypoint(mut self, name: &str, icon: WaypointIcon) -> Self {
        let player = self
            .world
            .levels
            .players()
            .find(|player| player.profile.name == name)
            .expect("fixture player");
        let target = CommandAttributeTarget::Player(
            crate::admin::NameAndId {
                name: (&player.profile.name).into(),
                id: player.profile.uuid,
            }
            .into(),
        );
        let dimension = player.dimension;
        self.world
            .apply_set_waypoint_icon_by_key(&dimension, target, icon, &mut Vec::new());
        self
    }
    pub(crate) fn player_waypoint_icon(&self, name: &str) -> &WaypointIcon {
        &self
            .world
            .levels
            .players()
            .find(|player| player.profile.name == name)
            .expect("fixture player")
            .cold
            .locator_bar_icon
    }
}
