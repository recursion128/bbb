use super::test_fixture::CommandTestWorld;
use crate::admin::{AdminState, NameAndId, OperatorEntry, StoredUserLists};
use crate::commands::{CommandDatapack, CommandPermission};

impl CommandTestWorld {
    fn with_user_lists(mut self, edit: impl FnOnce(&mut StoredUserLists)) -> Self {
        let mut lists = self.world.admin.lists().clone();
        edit(&mut lists);
        let features = self.world.cold.config.datapacks.as_ref().map_or_else(
            bbb_server_datapack::startup::default_enabled_features_26_1,
            |startup| startup.enabled_features.clone(),
        );
        let names: Vec<_> = features.iter().map(String::as_str).collect();
        self.world.admin = AdminState::new(self.world.admin.settings().clone(), lists, &names);
        self
    }
    pub(crate) fn with_operators(self, users: &[NameAndId]) -> Self {
        self.with_user_lists(|lists| {
            for user in users {
                lists.operators.insert(
                    user.id,
                    OperatorEntry {
                        user: user.clone(),
                        level: CommandPermission::Owner,
                        bypasses_player_limit: false,
                    },
                );
            }
        })
    }
    pub(crate) fn with_whitelist(mut self, users: &[NameAndId]) -> Self {
        for user in users {
            self.world.admin.add_whitelisted(user.clone());
        }
        self
    }
    /// `(user, source, reason)` per ban.
    pub(crate) fn with_player_bans(mut self, entries: &[(NameAndId, &str, Option<&str>)]) -> Self {
        for (user, source, reason) in entries {
            self.world.admin.add_player_ban(
                user.clone(),
                (*source).into(),
                reason.map(Into::into),
                0,
            );
        }
        self
    }
    /// `(address, source, reason)` per ban.
    pub(crate) fn with_ip_bans(mut self, entries: &[(&str, &str, Option<&str>)]) -> Self {
        for (address, source, reason) in entries {
            self.world.admin.add_ip_ban(
                (*address).to_owned(),
                (*source).into(),
                reason.map(Into::into),
                0,
            );
        }
        self
    }
    pub(crate) fn with_player_address(mut self, name: &str, address: &str) -> Self {
        let player = self
            .world
            .levels
            .players_mut()
            .find(|player| player.profile.name == name)
            .unwrap();
        player.cold.remote_address = Some(address.to_owned());
        self
    }
    pub(crate) fn with_force_game_mode(mut self, enabled: bool) -> Self {
        self.world.cold.config.force_game_mode = enabled;
        self
    }
    pub(crate) fn with_seed(mut self, seed: i64) -> Self {
        self.world.cold.seed = seed;
        self
    }
    pub(crate) fn with_debug_started(mut self) -> Self {
        self.setup("debug start");
        self
    }
    pub(crate) fn with_debug_sample(mut self, ticks: i32) -> Self {
        self.world.cold.debug_profiler_delay_start = false;
        self.world.cold.debug_profiler = Some((
            super::super::command_execution::debug_now_nanos() - 1_000_000_000,
            0,
        ));
        self.world.tick = ticks as u64;
        self
    }
    pub(crate) fn with_packs(
        mut self,
        available: &[CommandDatapack],
        selected: &[CommandDatapack],
    ) -> Self {
        let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
        startup.available = available
            .iter()
            .map(
                |pack| bbb_server_datapack::pack_repository::DatapackListing {
                    id: pack.id.clone(),
                    source: pack.source,
                    description_raw: bbb_registry_codec::RegistryJsonValue::String(
                        match &pack.description {
                            crate::commands::CommandPackDescription::Vanilla => {
                                "The default data for Minecraft".into()
                            }
                            crate::commands::CommandPackDescription::Literal(text) => text.clone(),
                        },
                    ),
                    requested_features: pack.requested_features.iter().cloned().collect(),
                },
            )
            .collect();
        startup.selection.ordered_ids = selected.iter().map(|pack| pack.id.clone()).collect();
        startup.selection.effective_data_packs.enabled = startup
            .selection
            .ordered_ids
            .iter()
            .map(Into::into)
            .collect();
        startup.selection.effective_data_packs.disabled = startup
            .available
            .iter()
            .filter(|pack| !startup.selection.ordered_ids.contains(&pack.id))
            .map(|pack| pack.id.clone().into())
            .collect();
        self.world.cold.config.datapacks = Some(std::sync::Arc::new(startup));
        self
    }
    pub(crate) fn with_datapack_directory(mut self, path: &std::path::Path) -> Self {
        let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
        startup.datapacks_dir = Some(path.to_owned());
        self.world.cold.config.datapacks = Some(std::sync::Arc::new(startup));
        self
    }
}
