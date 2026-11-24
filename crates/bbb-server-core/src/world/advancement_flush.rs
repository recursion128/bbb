//! The closing `ServerPlayer.tick` flush (`ServerPlayer.java:607`) and the
//! `PlayerList.reloadResources` store rebuild (`PlayerList.java:841-844`).

use super::{send, ServerWorld, WorldEvent};
use crate::commands::CommandAdvancementAction;
use crate::commands::CommandPlayerTarget;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_protocol::JavaString;
use bbb_server_datapack::player_advancements::{advancements_update_to_wire, AdvancementTree};
use bbb_server_persistence::Java26PlayerAdvancements;

impl ServerWorld {
    /// `ServerPlayer.tick`'s closing arm (`ServerPlayer.java:599-607`): after
    /// the experience packet, `flushDirty(this, true)` (`:607`).
    pub(super) fn tick_player_advancement_flush(&mut self, events: &mut Vec<WorldEvent>) {
        let mut dirty = Vec::new();
        for player in self.levels.players_mut() {
            if let Some(update) = player
                .cold
                .advancements
                .flush_dirty(&self.cold.advancement_tree, true)
            {
                dirty.push((player.connection_id, update));
            }
        }
        if dirty.is_empty() {
            return;
        }
        let resolver = self.item_stack_resolver();
        for (connection_id, update) in dirty {
            match advancements_update_to_wire(&update, &resolver) {
                Ok(packet) => {
                    self.levels[connection_id].cold.advancement_update_refusal = None;
                    events.push(send(
                        connection_id,
                        ClientboundPlayPacket::UpdateAdvancements(packet),
                    ));
                }
                Err(error) => {
                    self.levels[connection_id].cold.advancement_update_refusal = Some(error);
                }
            }
        }
    }

    /// `PlayerList.reloadResources` (`PlayerList.java:841-844`) after
    /// `saveAll` (`MinecraftServer.java:1539-1540`): each store is written back
    /// in memory, the tree is rebuilt from the installed generation, and every
    /// store `reload`s over that document (`PlayerAdvancements.java:85-95`).
    pub(super) fn reload_player_advancement_stores(&mut self) {
        let written: Vec<(ConnectionId, Java26PlayerAdvancements)> = self
            .levels
            .player_entries()
            .map(|(&connection_id, player)| {
                (
                    connection_id,
                    player
                        .cold
                        .advancements
                        .to_data(&self.cold.advancement_tree),
                )
            })
            .collect();
        self.cold.advancement_tree = self
            .cold
            .config
            .datapacks
            .as_deref()
            .map(|datapacks| {
                AdvancementTree::from_loaded_with_content_registry(
                    &datapacks.advancements,
                    self.registries.content_registry(),
                )
            })
            .unwrap_or_default();
        self.cold.advancement_triggers = super::advancement_triggers::AdvancementTriggers::compile(
            &self.cold.advancement_tree,
            self.cold.config.datapacks.as_deref(),
        );
        for (connection_id, data) in written {
            let Some(player) = self.levels.player_mut(connection_id) else {
                continue;
            };
            player
                .cold
                .advancements
                .reload(&self.cold.advancement_tree, Some(&data));
            player.cold.advancement_update_refusal = None;
        }
    }

    /// `Advancement.decorateName` (`Advancement.java:71-77`).
    pub(crate) fn decorate_advancement_name(
        display: &bbb_server_datapack::player_advancements::AdvancementDisplay,
    ) -> Result<bbb_protocol::NbtSummaryValue, String> {
        super::advancement_rewards::decorate_name(display)
    }

    /// `AdvancementCommands.Action.perform` / `performCriterion`
    /// (`AdvancementCommands.java:220-336`, `:417-434`).
    #[cfg(test)]
    pub(crate) fn apply_mutate_advancements(
        &mut self,
        player: CommandPlayerTarget,
        action: CommandAdvancementAction,
        ids: Vec<String>,
        criterion: Option<JavaString>,
        show_advancements: bool,
        events: &mut Vec<WorldEvent>,
    ) -> usize {
        self.apply_mutate_advancements_slice(
            player,
            action,
            &ids,
            criterion.as_ref(),
            show_advancements,
            events,
        )
    }

    pub(crate) fn apply_mutate_advancements_slice(
        &mut self,
        player: CommandPlayerTarget,
        action: CommandAdvancementAction,
        ids: &[String],
        criterion: Option<&JavaString>,
        show_advancements: bool,
        events: &mut Vec<WorldEvent>,
    ) -> usize {
        let Some(connection_id) = self.command_player_connection(&player) else {
            return 0;
        };
        let epoch = self.cold.wall_clock_millis;
        if !show_advancements {
            self.flush_player_advancements(connection_id, true, events);
        }
        let mut mutated = 0;
        if let Some(criterion) = criterion {
            if let Some(id) = ids.first() {
                if self.mutate_advancement_criterion(
                    connection_id,
                    action,
                    id,
                    criterion,
                    epoch,
                    events,
                ) {
                    mutated += 1;
                }
            }
        } else {
            for id in ids {
                if self.mutate_advancement(connection_id, action, id, epoch, events) {
                    mutated += 1;
                }
            }
        }
        if !show_advancements {
            self.flush_player_advancements(connection_id, false, events);
        }
        mutated
    }

    fn flush_player_advancements(
        &mut self,
        connection_id: ConnectionId,
        show_advancements: bool,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        let Some(update) = player
            .cold
            .advancements
            .flush_dirty(&self.cold.advancement_tree, show_advancements)
        else {
            return;
        };
        match advancements_update_to_wire(&update, &resolver) {
            Ok(packet) => {
                player.cold.advancement_update_refusal = None;
                events.push(send(
                    player.connection_id,
                    ClientboundPlayPacket::UpdateAdvancements(packet),
                ));
            }
            Err(error) => player.cold.advancement_update_refusal = Some(error),
        }
    }

    fn mutate_advancement(
        &mut self,
        connection_id: ConnectionId,
        action: CommandAdvancementAction,
        id: &str,
        epoch: i64,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(index) = self.cold.advancement_tree.index_of(id) else {
            return false;
        };
        let tree = &self.cold.advancement_tree;
        let Some(player) = self.levels.player_mut(connection_id) else {
            return false;
        };
        match action {
            // `Action.GRANT.perform` (`AdvancementCommands.java:372-383`).
            CommandAdvancementAction::Grant => {
                let remaining: Vec<JavaString> = {
                    let store = &mut player.cold.advancements;
                    let progress = store.get_or_start_progress(tree, index);
                    if progress.is_done() {
                        return false;
                    }
                    progress.remaining_criteria().into_iter().cloned().collect()
                };
                for criterion in remaining {
                    self.award_advancement_criterion(
                        connection_id,
                        index,
                        &criterion,
                        epoch,
                        events,
                    );
                }
                true
            }
            // `Action.REVOKE.perform` (`AdvancementCommands.java:392-403`).
            CommandAdvancementAction::Revoke => {
                let completed: Vec<JavaString> = {
                    let store = &mut player.cold.advancements;
                    let progress = store.get_or_start_progress(tree, index);
                    if !progress.has_progress() {
                        return false;
                    }
                    progress.completed_criteria().into_iter().cloned().collect()
                };
                for criterion in completed {
                    player.cold.advancements.revoke(tree, index, &criterion);
                }
                true
            }
        }
    }

    fn mutate_advancement_criterion(
        &mut self,
        connection_id: ConnectionId,
        action: CommandAdvancementAction,
        id: &str,
        criterion: &JavaString,
        epoch: i64,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let Some(index) = self.cold.advancement_tree.index_of(id) else {
            return false;
        };
        let tree = &self.cold.advancement_tree;
        let Some(player) = self.levels.player_mut(connection_id) else {
            return false;
        };
        match action {
            // `Action.GRANT.performCriterion` (`AdvancementCommands.java:386-388`).
            CommandAdvancementAction::Grant => {
                self.award_advancement_criterion(connection_id, index, criterion, epoch, events)
            }
            // `Action.REVOKE.performCriterion` (`AdvancementCommands.java:406-408`).
            CommandAdvancementAction::Revoke => {
                player.cold.advancements.revoke(tree, index, criterion)
            }
        }
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn seed_restored_player_advancements(
        &mut self,
        uuid: uuid::Uuid,
        data: Java26PlayerAdvancements,
    ) {
        self.cold.restored_player_advancements.insert(uuid, data);
    }

    pub(crate) fn player_advancements_to_data(
        &self,
        connection_id: ConnectionId,
    ) -> Java26PlayerAdvancements {
        self.levels[connection_id]
            .cold
            .advancements
            .to_data(&self.cold.advancement_tree)
    }

    /// `PlayerAdvancements.award` (`PlayerAdvancements.java:167-190`) on a live
    /// session, in place of the criterion trigger that would fire it.
    pub(crate) fn award_player_advancement_for_test(
        &mut self,
        connection_id: ConnectionId,
        id: &str,
        criterion: &str,
        epoch_millis: i64,
    ) -> bool {
        let index = self
            .cold
            .advancement_tree
            .index_of(id)
            .expect("the test names a loaded advancement");
        let tree = &self.cold.advancement_tree;
        self.levels[connection_id]
            .cold
            .advancements
            .award(tree, index, &criterion.into(), epoch_millis)
            .granted
    }

    pub(crate) fn player_advancement_is_done(&self, connection_id: ConnectionId, id: &str) -> bool {
        let Some(index) = self.cold.advancement_tree.index_of(id) else {
            return false;
        };
        self.levels[connection_id]
            .cold
            .advancements
            .progress(index)
            .is_some_and(|progress| progress.is_done())
    }
}
