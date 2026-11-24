//! `ServerScoreboard` (`ServerScoreboard.java:27-34`): the live scoreboard plus
//! the fan-out that turns every mutation into packets.
//!
//! Vanilla puts the fan-out in a subclass whose `on*` overrides the base
//! [`Scoreboard`] calls from inside its own mutators. Rust has no such
//! override, so the split is a wrapper instead: every mutator a command reaches
//! is here, each one calls the base mutator and then the same callback vanilla's
//! override would have run. Nothing outside this file may mutate the wrapped
//! board, which is what keeps a change from reaching the world without reaching
//! the clients.
//!
//! `broadcastAll` is not called here either. The packets a mutation produced
//! are queued on [`ServerScoreboard::pending`] and drained by the world, which
//! is the only thing that holds the player list.

use crate::connection::ClientboundPlayPacket;
use bbb_protocol::objective_render_types::ObjectiveRenderType;
use bbb_protocol::packets::{
    NumberFormatSummary, PlayerTeamMethod, PlayerTeamParameters, ResetScore, ScoreboardDisplaySlot,
    SetDisplayObjective, SetObjective, SetObjectiveMethod, SetObjectiveParameters, SetPlayerTeam,
    SetScore,
};
use bbb_protocol::{JavaString, NbtSummaryValue, RichTextComponent, RichTextStyle};
use bbb_server_persistence::{Java26ScoreboardNumberFormat, Java26ScoreboardPacked};

use super::scoreboard::{
    Objective, PlayerTeam, Score, ScoreResetOutcome, Scoreboard, ScoreboardName,
};

/// `ServerScoreboard` (`ServerScoreboard.java:27-30`) minus `dirty`, which
/// gates `storeToSaveDataIfDirty` (`:185-194`); this world packs the whole
/// scoreboard on every save, so the flag would decide nothing.
#[derive(Clone, Debug)]
pub(crate) struct ServerScoreboard {
    board: Scoreboard,
    /// `trackedObjectives` (`:29`), by name: the objectives at least one
    /// display slot shows, and so the ones whose changes reach a client.
    tracked_objectives: Vec<JavaString>,
    /// What `this.server.getPlayerList().broadcastAll(..)` was called with,
    /// held until the world drains it.
    pending: Vec<ClientboundPlayPacket>,
    pending_waypoint_updates: Vec<(usize, JavaString, Option<u32>, bool)>,
    /// Destroyed entities whose `entityRemoved` waits for the drain, so a
    /// conversion's queued team transfer still reads the source's team.
    pending_entity_removals: Vec<String>,
    content_registry: std::sync::Arc<bbb_protocol::content_registry::ContentRegistry>,
}

impl Default for ServerScoreboard {
    fn default() -> Self {
        Self::with_content_registry(std::sync::Arc::clone(
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        ))
    }
}

pub(crate) enum ServerScoreboardEffect {
    Packet(Box<ClientboundPlayPacket>),
    RemakePlayerWaypoint {
        player: JavaString,
        team_color_rgb: Option<u32>,
        all_levels: bool,
    },
}

impl ServerScoreboard {
    pub(crate) fn with_content_registry(
        content_registry: std::sync::Arc<bbb_protocol::content_registry::ContentRegistry>,
    ) -> Self {
        Self {
            board: Scoreboard::default(),
            tracked_objectives: Vec::new(),
            pending: Vec::new(),
            pending_waypoint_updates: Vec::new(),
            pending_entity_removals: Vec::new(),
            content_registry,
        }
    }

    pub(crate) fn board(&self) -> &Scoreboard {
        &self.board
    }

    pub(crate) fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        &self.content_registry
    }

    /// The packets every mutation since the last drain produced, in the order
    /// vanilla sent them.
    #[cfg(test)]
    pub(crate) fn take_pending(&mut self) -> Vec<ClientboundPlayPacket> {
        self.pending_waypoint_updates.clear();
        std::mem::take(&mut self.pending)
    }

    /// `Scoreboard.entityRemoved` (`Scoreboard.java:342-347`) for a destroyed
    /// non-player entity, applied at the next drain.
    pub(crate) fn queue_entity_removed(&mut self, name: String) {
        self.pending_entity_removals.push(name);
    }

    pub(crate) fn take_pending_effects(&mut self) -> Vec<ServerScoreboardEffect> {
        if self.pending_entity_removals.is_empty()
            && self.pending.is_empty()
            && self.pending_waypoint_updates.is_empty()
        {
            return Vec::new();
        }
        for name in std::mem::take(&mut self.pending_entity_removals) {
            self.reset_all_scores(&name);
            self.remove_player_from_team(&name);
        }
        let mut packets = std::mem::take(&mut self.pending).into_iter();
        let updates = std::mem::take(&mut self.pending_waypoint_updates);
        let mut effects = Vec::with_capacity(packets.len() + updates.len());
        let mut updates = updates.into_iter().peekable();
        for sent in 0..=packets.len() {
            while updates
                .peek()
                .is_some_and(|(after, _, _, _)| *after == sent)
            {
                let (_, player, team_color_rgb, all_levels) =
                    updates.next().expect("peeked update exists");
                effects.push(ServerScoreboardEffect::RemakePlayerWaypoint {
                    player,
                    team_color_rgb,
                    all_levels,
                });
            }
            if let Some(packet) = packets.next() {
                effects.push(ServerScoreboardEffect::Packet(Box::new(packet)));
            }
        }
        effects
    }

    /// `ServerScoreboard.load` (`:36-44`) through `ScoreboardSaveData`.
    ///
    /// Vanilla loads before any session is placed, so the display-slot writes
    /// this walk makes start tracking without anything to send to. Draining
    /// here says the same thing: what the load queued reaches nobody.
    #[cfg(test)]
    pub(crate) fn restore(packed: &Java26ScoreboardPacked) -> Self {
        Self::restore_with_registry(
            packed,
            std::sync::Arc::clone(bbb_protocol::content_registry::ContentRegistry::vanilla()),
        )
    }

    pub(crate) fn restore_with_registry(
        packed: &Java26ScoreboardPacked,
        content_registry: std::sync::Arc<bbb_protocol::content_registry::ContentRegistry>,
    ) -> Self {
        let mut scoreboard = Self {
            board: Scoreboard::restore(packed),
            tracked_objectives: Vec::new(),
            pending: Vec::new(),
            pending_waypoint_updates: Vec::new(),
            pending_entity_removals: Vec::new(),
            content_registry,
        };
        let displayed = scoreboard
            .board
            .display_objectives()
            .iter()
            .map(|(_, objective)| objective.clone())
            .collect::<Vec<_>>();
        for objective in displayed {
            if !scoreboard
                .tracked_objectives
                .iter()
                .any(|name| objective.matches(name))
            {
                scoreboard.start_tracking_objective(&objective);
            }
        }
        scoreboard.pending.clear();
        scoreboard.pending_waypoint_updates.clear();
        scoreboard
    }

    pub(crate) fn pack(&self) -> Java26ScoreboardPacked {
        self.board.pack()
    }

    /// `addObjective` (`Scoreboard.java:44-61`) as `ScoreboardCommand.addObjective`
    /// calls it (`ScoreboardCommand.java:849`): auto-update off and no number
    /// format. `onObjectiveAdded` (`ServerScoreboard.java:137-140`) sends
    /// nothing, because no display slot tracks a fresh objective.
    pub(crate) fn add_objective(
        &mut self,
        name: &(impl ScoreboardName + ?Sized),
        criteria: &str,
        display_name: NbtSummaryValue,
        render_type: ObjectiveRenderType,
    ) {
        self.board.add_objective(Objective {
            name: name.to_java(),
            criteria: criteria.to_owned(),
            display_name,
            render_type,
            display_auto_update: false,
            number_format: None,
        });
    }

    /// One `Objective` setter (`Objective.java:87-110`) and the
    /// `onObjectiveChanged` it ends with.
    pub(crate) fn modify_objective(
        &mut self,
        name: &(impl ScoreboardName + ?Sized),
        write: impl FnOnce(&mut Objective),
    ) {
        if let Some(entry) = self.board.objective_mut(name) {
            write(entry);
            self.on_objective_changed(name);
        }
    }

    /// `ScoreAccess.set` (`Scoreboard.java:87-109`). `display_name` is
    /// `ScoreHolder.getDisplayName()` (`:94`), which only an auto-updating
    /// objective reads.
    pub(crate) fn set_score(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        display_name: Option<&NbtSummaryValue>,
        objective: &(impl ScoreboardName + ?Sized),
        value: i32,
    ) {
        self.write_score(owner, display_name, objective, false, |_| value);
    }

    /// `ScoreAccess.display` (`:116-122`).
    pub(crate) fn set_score_display(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
        display: Option<NbtSummaryValue>,
    ) {
        let Some((score, created)) = self.board.score_entry_mut(owner, objective) else {
            return;
        };
        if created || score.display != display {
            score.display = display;
            self.on_score_changed(owner, objective);
        }
    }

    /// `ScoreAccess.numberFormatOverride` (`:124-128`), which sends whether or
    /// not the value changed.
    pub(crate) fn set_score_number_format(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
        format: Option<Java26ScoreboardNumberFormat>,
    ) {
        let Some((score, _)) = self.board.score_entry_mut(owner, objective) else {
            return;
        };
        score.number_format = format;
        self.on_score_changed(owner, objective);
    }

    /// `getOrCreatePlayerScore` (`:71-79`) on its own, which files the entry
    /// before any `ScoreAccess` method runs.
    pub(crate) fn track_score(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
    ) {
        self.board.score_entry_mut(owner, objective);
    }

    /// `ScoreAccess.lock` / `unlock` through `setLocked` (`:136-152`), whose
    /// own send only fires for a score this access filed.
    pub(crate) fn set_score_locked(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
        locked: bool,
    ) {
        let Some((score, created)) = self.board.score_entry_mut(owner, objective) else {
            return;
        };
        score.locked = locked;
        if created {
            self.on_score_changed(owner, objective);
        }
    }

    /// `resetAllPlayerScores` (`:189-194`) into `onPlayerRemoved`
    /// (`ServerScoreboard.java:77-81`).
    pub(crate) fn reset_all_scores(&mut self, owner: &(impl ScoreboardName + ?Sized)) {
        if self.board.reset_all_scores(owner) {
            self.pending.push(reset_score_packet(owner, None));
        }
    }

    /// `resetSinglePlayerScore` (`:196-209`).
    pub(crate) fn reset_score(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
    ) {
        match self.board.reset_score(owner, objective) {
            ScoreResetOutcome::OwnerRemoved => {
                self.pending.push(reset_score_packet(owner, None));
            }
            ScoreResetOutcome::ScoreRemoved => {
                if self
                    .tracked_objectives
                    .iter()
                    .any(|name| objective.matches(name))
                {
                    self.pending
                        .push(reset_score_packet(owner, Some(objective.to_java())));
                }
            }
            ScoreResetOutcome::Nothing => {}
        }
    }

    /// `addPlayerTeam` (`Scoreboard.java:249-260`) into `onTeamAdded`
    /// (`ServerScoreboard.java:163-167`).
    pub(crate) fn add_team(&mut self, name: &(impl ScoreboardName + ?Sized)) {
        if self.board.add_team(name) {
            let team = self.board.team(name).expect("the team was just added");
            self.pending
                .push(team_packet(team, true, &self.content_registry));
        }
    }

    /// `removePlayerTeam` (`:262-270`) into `onTeamRemoved`
    /// (`ServerScoreboard.java:178-183`); the members' scores stay.
    pub(crate) fn remove_team(&mut self, name: &(impl ScoreboardName + ?Sized)) {
        let players: Vec<_> = self.board.team_members(name).cloned().collect();
        if self.board.remove_team(name) {
            self.pending.push(remove_team_packet(name));
            for player in players {
                self.queue_waypoint_update(player, None, true);
            }
        }
    }

    /// `ServerScoreboard.addPlayerToTeam` (`:117-126`): a player on any team,
    /// this one included, draws a LEAVE before the JOIN (`:129-134`).
    pub(crate) fn add_player_to_team(
        &mut self,
        player: &(impl ScoreboardName + ?Sized),
        team: &(impl ScoreboardName + ?Sized),
    ) {
        let Some(join) = self.board.add_player_to_team(player, team) else {
            return;
        };
        if let Some(left) = &join.left {
            self.pending
                .push(team_member_packet(left, player, PlayerTeamMethod::Leave));
            self.queue_waypoint_update(player.to_java(), None, false);
        }
        if join.added {
            self.pending
                .push(team_member_packet(team, player, PlayerTeamMethod::Join));
            let team_color_rgb = self.board.team(team).and_then(|team| team.color.color());
            self.queue_waypoint_update(player.to_java(), team_color_rgb, false);
        }
    }

    /// `removePlayerFromTeam(String)` (`:281-289`).
    pub(crate) fn remove_player_from_team(&mut self, player: &(impl ScoreboardName + ?Sized)) {
        if let Some(left) = self.board.remove_player_from_team(player) {
            self.pending
                .push(team_member_packet(&left, player, PlayerTeamMethod::Leave));
            self.queue_waypoint_update(player.to_java(), None, false);
        }
    }

    /// `removePlayerFromTeam(String, PlayerTeam)` (`:291-298`).
    pub(crate) fn remove_player_from_named_team(
        &mut self,
        player: &(impl ScoreboardName + ?Sized),
        team: &(impl ScoreboardName + ?Sized),
    ) {
        if self.board.remove_player_from_named_team(player, team) {
            self.pending
                .push(team_member_packet(team, player, PlayerTeamMethod::Leave));
            self.queue_waypoint_update(player.to_java(), None, false);
        }
    }

    /// One `PlayerTeam` setter and the `onTeamChanged` every one of the eight
    /// ends with (`PlayerTeam.java:83-201`, `ServerScoreboard.java:170-175`).
    /// None of them guards on the value it is given, so a setter that writes
    /// what was already there still broadcasts.
    pub(crate) fn change_team(
        &mut self,
        name: &(impl ScoreboardName + ?Sized),
        write: impl FnOnce(&mut PlayerTeam),
    ) {
        let Some(entry) = self.board.team_mut(name) else {
            return;
        };
        write(entry);
        let team = self.board.team(name).expect("the team was just written");
        let players: Vec<_> = self.board.team_members(&team.name).cloned().collect();
        let team_color_rgb = team.color.color();
        self.pending
            .push(team_packet(team, false, &self.content_registry));
        for player in players {
            self.queue_waypoint_update(player, team_color_rgb, true);
        }
    }

    fn queue_waypoint_update(
        &mut self,
        player: JavaString,
        team_color_rgb: Option<u32>,
        all_levels: bool,
    ) {
        self.pending_waypoint_updates.push((
            self.pending.len(),
            player,
            team_color_rgb,
            all_levels,
        ));
    }

    /// `Scoreboard.removeObjective` (`Scoreboard.java:216-235`), whose
    /// display-slot loop goes through the overridden `setDisplayObjective`
    /// (`ServerScoreboard.java:94-114`) and so stops the tracking before
    /// `onObjectiveRemoved` (`:153-160`) is reached.
    pub(crate) fn remove_objective(&mut self, name: &(impl ScoreboardName + ?Sized)) {
        if self.board.objective(name).is_none() {
            return;
        }
        for slot in displayed_slots(&self.board, name) {
            self.set_display_objective(slot, None);
        }
        self.board.remove_objective(name);
        if self
            .tracked_objectives
            .iter()
            .any(|entry| name.matches(entry))
        {
            self.stop_tracking_objective(name);
        }
    }

    /// `ServerScoreboard.setDisplayObjective` (`:94-114`).
    pub(crate) fn set_display_objective(
        &mut self,
        slot: ScoreboardDisplaySlot,
        objective: Option<&JavaString>,
    ) {
        let old = self.board.display_objective(slot).cloned();
        self.board.set_display_objective(slot, objective);
        if let Some(old) = old.filter(|old| Some(old) != objective) {
            if displayed_slots(&self.board, &old).is_empty() {
                self.stop_tracking_objective(&old);
            } else {
                self.pending
                    .push(display_objective_packet(slot, objective.cloned()));
            }
        }
        if let Some(name) = objective {
            if self
                .tracked_objectives
                .iter()
                .any(|entry| name.matches(entry))
            {
                self.pending
                    .push(display_objective_packet(slot, Some(name.to_java())));
            } else {
                self.start_tracking_objective(name);
            }
        }
    }

    /// `Scoreboard.forAllObjectives` (`Scoreboard.java:63-65`): every objective
    /// on `criteria`, in add order, through the `forceWritable` score access, so
    /// a read-only criterion is written too. `value` maps the current score to
    /// the new one, which covers `set` and `ScoreAccess.increment`
    /// (`ScoreAccess.java:12-20`).
    pub(crate) fn for_all_objectives(
        &mut self,
        criteria: &str,
        owner: &(impl ScoreboardName + ?Sized),
        display_name: Option<&NbtSummaryValue>,
        value: impl Fn(i32) -> i32,
    ) {
        let count = self.board.objective_indices_for_criteria(criteria).len();
        for pos in 0..count {
            let index = self.board.objective_indices_for_criteria(criteria)[pos];
            let name = self.board.objectives()[index].name.clone();
            self.write_score(owner, display_name, &name, true, &value);
        }
    }

    /// Whether a [`Self::for_all_objectives`] write on `criteria` would read the
    /// holder's display name (`Scoreboard.java:93-98`).
    pub(crate) fn criteria_reads_display_name(&self, criteria: &str) -> bool {
        self.board
            .objectives_for_criteria(criteria)
            .any(|objective| objective.display_auto_update)
    }

    /// `ScoreAccess.set` (`Scoreboard.java:87-109`) over the two-argument
    /// `getOrCreatePlayerScore` (`:71-73`): the auto-update write, the value
    /// write, and the one send both of them share. A read-only criterion is
    /// skipped unless `force_writable`; commands refuse it before writing.
    fn write_score(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        display_name: Option<&NbtSummaryValue>,
        objective: &(impl ScoreboardName + ?Sized),
        force_writable: bool,
        value: impl FnOnce(i32) -> i32,
    ) {
        let Some(entry) = self.board.objective(objective) else {
            return;
        };
        let auto_update = entry.display_auto_update;
        let writable =
            force_writable || !crate::world::scoreboard::criteria_facts(&entry.criteria).read_only;
        let Some((score, created)) = self.board.score_entry_mut(owner, objective) else {
            return;
        };
        if !writable {
            return;
        }
        let mut changed = created;
        if auto_update {
            if let Some(display) = display_name {
                if score.display.as_ref() != Some(display) {
                    score.display = Some(display.clone());
                    changed = true;
                }
            }
        }
        let value = value(score.value);
        if score.value != value {
            score.value = value;
            changed = true;
        }
        if changed {
            self.on_score_changed(owner, objective);
        }
    }

    /// `onScoreChanged` (`ServerScoreboard.java:51-68`).
    fn on_score_changed(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
    ) {
        if !self
            .tracked_objectives
            .iter()
            .any(|name| objective.matches(name))
        {
            return;
        }
        let Some(score) = self.board.score(owner, objective) else {
            return;
        };
        self.pending.push(score_packet(
            owner,
            objective,
            score,
            &self.content_registry,
        ));
    }

    /// `onObjectiveChanged` (`ServerScoreboard.java:143-150`).
    fn on_objective_changed(&mut self, name: &(impl ScoreboardName + ?Sized)) {
        if !self
            .tracked_objectives
            .iter()
            .any(|entry| name.matches(entry))
        {
            return;
        }
        let objective = self
            .board
            .objective(name)
            .expect("the objective was just modified");
        self.pending.push(objective_packet(
            objective,
            SetObjectiveMethod::Change,
            &self.content_registry,
        ));
    }

    /// `getStartTrackingPackets` (`ServerScoreboard.java:196-215`).
    pub(crate) fn start_tracking_packets(
        &self,
        name: &(impl ScoreboardName + ?Sized),
    ) -> Vec<ClientboundPlayPacket> {
        let Some(objective) = self.board.objective(name) else {
            return Vec::new();
        };
        let mut packets = vec![objective_packet(
            objective,
            SetObjectiveMethod::Add,
            &self.content_registry,
        )];
        for slot in displayed_slots(&self.board, name) {
            packets.push(display_objective_packet(slot, Some(name.to_java())));
        }
        for (owner, score) in self.board.scores_for(name) {
            packets.push(score_packet(owner, name, score, &self.content_registry));
        }
        packets
    }

    /// `startTrackingObjective` (`ServerScoreboard.java:217-227`).
    fn start_tracking_objective(&mut self, name: &(impl ScoreboardName + ?Sized)) {
        self.pending.extend(self.start_tracking_packets(name));
        self.tracked_objectives.push(name.to_java());
    }

    /// `getStopTrackingPackets` (`:229-240`) and `stopTrackingObjective`
    /// (`:242-252`).
    fn stop_tracking_objective(&mut self, name: &(impl ScoreboardName + ?Sized)) {
        if let Some(objective) = self.board.objective(name) {
            self.pending.push(objective_packet(
                objective,
                SetObjectiveMethod::Remove,
                &self.content_registry,
            ));
            // The display-slot arm (`:233-237`), which names the objective
            // being dropped rather than clearing the slot.
            for slot in displayed_slots(&self.board, name) {
                self.pending
                    .push(display_objective_packet(slot, Some(name.to_java())));
            }
        }
        self.tracked_objectives.retain(|entry| !name.matches(entry));
    }

    /// `PlayerList.updateEntireScoreboard` (`PlayerList.java:220-237`): every
    /// team, then one start-tracking run per objective a display slot shows.
    pub(crate) fn join_packets(&self) -> Vec<ClientboundPlayPacket> {
        const TEAM_JOIN_BATCH_SIZE: usize = 8_192;
        let teams = self.board.teams();
        let mut grouped: Vec<Vec<JavaString>> = vec![Vec::new(); teams.len()];
        for (member, team) in &self.board.teams.members {
            grouped[*team as usize].push(member.clone());
        }
        let mut packets = Vec::with_capacity(teams.len());
        for (team, members) in teams.iter().zip(grouped) {
            let mut packet = team_packet(team, true, &self.content_registry);
            let mut iter = members.into_iter();
            if let ClientboundPlayPacket::SetPlayerTeam(ref mut add_packet) = packet {
                add_packet
                    .players
                    .extend(iter.by_ref().take(TEAM_JOIN_BATCH_SIZE));
            }
            packets.push(packet);
            loop {
                let batch: Vec<JavaString> = iter.by_ref().take(TEAM_JOIN_BATCH_SIZE).collect();
                if batch.is_empty() {
                    break;
                }
                packets.push(ClientboundPlayPacket::SetPlayerTeam(SetPlayerTeam {
                    name: team.name.clone(),
                    method: PlayerTeamMethod::Join.into(),
                    parameters: None,
                    players: batch,
                }));
            }
        }
        let mut seen = Vec::new();
        for (_, objective) in self.board.display_objectives() {
            if seen.iter().any(|name| objective.matches(name)) {
                continue;
            }
            packets.extend(self.start_tracking_packets(objective));
            seen.push(objective.clone());
        }
        packets
    }
}

fn displayed_slots(
    board: &Scoreboard,
    objective: &(impl ScoreboardName + ?Sized),
) -> Vec<ScoreboardDisplaySlot> {
    board
        .display_objectives()
        .iter()
        .filter(|(_, name)| objective.matches(name))
        .map(|(slot, _)| *slot)
        .collect()
}

/// `new ClientboundSetObjectivePacket(objective, method)`
/// (`ClientboundSetObjectivePacket.java:29-35`).
fn objective_packet(
    objective: &Objective,
    method: SetObjectiveMethod,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetObjective(SetObjective {
        objective_name: objective.name.clone(),
        method: method.into(),
        parameters: matches!(method, SetObjectiveMethod::Add | SetObjectiveMethod::Change).then(
            || SetObjectiveParameters {
                display_name: component(objective.display_name.clone(), content_registry),
                render_type: objective.render_type,
                number_format: objective
                    .number_format
                    .as_ref()
                    .map(|format| number_format(format, content_registry)),
            },
        ),
    })
}

/// `new ClientboundSetScorePacket(..)` (`ServerScoreboard.java:57-63`).
fn score_packet(
    owner: &(impl ScoreboardName + ?Sized),
    objective: &(impl ScoreboardName + ?Sized),
    score: &Score,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetScore(SetScore {
        owner: owner.to_java(),
        objective_name: objective.to_java(),
        score: score.value,
        display: score
            .display
            .clone()
            .map(|value| component(value, content_registry)),
        number_format: score
            .number_format
            .as_ref()
            .map(|format| number_format(format, content_registry)),
    })
}

/// `new ClientboundResetScorePacket(owner, objective)`
/// (`ServerScoreboard.java:79`, `:87`).
fn reset_score_packet(
    owner: &(impl ScoreboardName + ?Sized),
    objective: Option<JavaString>,
) -> ClientboundPlayPacket {
    ClientboundPlayPacket::ResetScore(ResetScore {
        owner: owner.to_java(),
        objective_name: objective,
    })
}

/// `new ClientboundSetDisplayObjectivePacket(slot, objective)`
/// (`ServerScoreboard.java:99`, `:107`).
fn display_objective_packet(
    slot: ScoreboardDisplaySlot,
    objective: Option<JavaString>,
) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetDisplayObjective(SetDisplayObjective {
        slot,
        objective_name: objective,
    })
}

/// `ClientboundSetPlayerTeamPacket.createAddOrModifyPacket`
/// (`ClientboundSetPlayerTeamPacket.java:43-50`) over `Parameters(team)`
/// (`:151-159`).
fn team_packet(
    team: &PlayerTeam,
    create_new: bool,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetPlayerTeam(SetPlayerTeam {
        name: team.name.clone(),
        method: if create_new {
            PlayerTeamMethod::Add
        } else {
            PlayerTeamMethod::Change
        }
        .into(),
        parameters: Some(PlayerTeamParameters {
            display_name: component(team.display_name.clone(), content_registry),
            // `packOptions` (`PlayerTeam.java:180-191`).
            options: u8::from(team.allow_friendly_fire)
                | (u8::from(team.see_friendly_invisibles) << 1),
            nametag_visibility: team.name_tag_visibility,
            collision_rule: team.collision_rule,
            color: team.color,
            player_prefix: component(
                team.player_prefix
                    .clone()
                    .unwrap_or_else(empty_component_nbt),
                content_registry,
            ),
            player_suffix: component(
                team.player_suffix
                    .clone()
                    .unwrap_or_else(empty_component_nbt),
                content_registry,
            ),
        }),
        players: Vec::new(),
    })
}

/// `ClientboundSetPlayerTeamPacket.createRemovePacket`
/// (`ClientboundSetPlayerTeamPacket.java:52-54`), which carries neither
/// parameters nor a member list.
fn remove_team_packet(name: &(impl ScoreboardName + ?Sized)) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetPlayerTeam(SetPlayerTeam {
        name: name.to_java(),
        method: PlayerTeamMethod::Remove.into(),
        parameters: None,
        players: Vec::new(),
    })
}

/// `createPlayerPacket` (`:56-62`): the one member, and no parameters at all.
fn team_member_packet(
    team: &(impl ScoreboardName + ?Sized),
    player: &(impl ScoreboardName + ?Sized),
    method: PlayerTeamMethod,
) -> ClientboundPlayPacket {
    ClientboundPlayPacket::SetPlayerTeam(SetPlayerTeam {
        name: team.to_java(),
        method: method.into(),
        parameters: None,
        players: vec![player.to_java()],
    })
}

/// `CommonComponents.EMPTY` (`PlayerTeam.java:27-28`), the prefix and suffix a
/// team that was never given one carries.
fn empty_component_nbt() -> NbtSummaryValue {
    NbtSummaryValue::string(String::new())
}

fn component(
    value: NbtSummaryValue,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> RichTextComponent {
    RichTextComponent::try_from_component_nbt_with_registry(value, content_registry)
        .expect("every stored scoreboard component was validated when it was written")
}

/// `NumberFormatTypes` (`NumberFormatTypes.java:14-17`) as the packet carries it.
fn number_format(
    format: &Java26ScoreboardNumberFormat,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> NumberFormatSummary {
    match format {
        Java26ScoreboardNumberFormat::Blank => NumberFormatSummary::Blank,
        Java26ScoreboardNumberFormat::Styled(style) => NumberFormatSummary::Styled(
            RichTextStyle::try_from_style_nbt_with_registry(style.clone(), content_registry)
                .expect("every stored styled format was validated when it was written"),
        ),
        Java26ScoreboardNumberFormat::Fixed(value) => {
            NumberFormatSummary::Fixed(component(value.clone(), content_registry))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::chat_formattings::ChatFormatting;
    use bbb_protocol::objective_render_types::ObjectiveRenderType;

    fn text(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::string(value)
    }

    fn scoreboard(objectives: &[&str]) -> ServerScoreboard {
        let mut scoreboard = ServerScoreboard::default();
        for name in objectives {
            scoreboard.add_objective(*name, "dummy", text(name), ObjectiveRenderType::Integer);
        }
        scoreboard
    }

    #[test]
    fn team_packets_precede_waypoint_remakes_with_the_transition_color_and_scope() {
        let mut scoreboard = ServerScoreboard::default();
        for team in ["red", "blue"] {
            scoreboard.add_team(team);
        }
        scoreboard.change_team("red", |team| team.color = ChatFormatting::Red);
        scoreboard.change_team("blue", |team| team.color = ChatFormatting::Blue);
        scoreboard.take_pending_effects();

        scoreboard.add_player_to_team("bob", "red");
        let effects = scoreboard.take_pending_effects();
        assert!(matches!(effects[0], ServerScoreboardEffect::Packet(_)));
        assert!(matches!(
            effects[1],
            ServerScoreboardEffect::RemakePlayerWaypoint {
                ref player,
                team_color_rgb,
                all_levels: false,
            } if player == "bob" && team_color_rgb == ChatFormatting::Red.color()
        ));

        scoreboard.add_player_to_team("bob", "blue");
        let effects = scoreboard.take_pending_effects();
        assert_eq!(effects.len(), 4);
        assert!(matches!(effects[0], ServerScoreboardEffect::Packet(_)));
        assert!(matches!(
            effects[1],
            ServerScoreboardEffect::RemakePlayerWaypoint {
                ref player,
                team_color_rgb: None,
                all_levels: false,
            } if player == "bob"
        ));
        assert!(matches!(effects[2], ServerScoreboardEffect::Packet(_)));
        assert!(matches!(
            effects[3],
            ServerScoreboardEffect::RemakePlayerWaypoint {
                ref player,
                team_color_rgb,
                all_levels: false,
            } if player == "bob" && team_color_rgb == ChatFormatting::Blue.color()
        ));

        scoreboard.change_team("blue", |team| team.color = ChatFormatting::Black);
        let effects = scoreboard.take_pending_effects();
        assert!(matches!(effects[0], ServerScoreboardEffect::Packet(_)));
        assert!(matches!(
            effects[1],
            ServerScoreboardEffect::RemakePlayerWaypoint {
                ref player,
                team_color_rgb: Some(0),
                all_levels: true,
            } if player == "bob"
        ));
    }

    /// `onObjectiveAdded` (`ServerScoreboard.java:137-140`) and `onScoreChanged`
    /// (`:51-68`) both check `trackedObjectives` before they broadcast, and
    /// nothing is tracked until a display slot shows it (`:105-110`).
    #[test]
    fn nothing_reaches_a_client_until_a_display_slot_tracks_the_objective() {
        let mut scoreboard = scoreboard(&["kills"]);
        scoreboard.set_score("bob", None, "kills", 4);
        assert_eq!(scoreboard.take_pending(), Vec::new());

        // `startTrackingObjective` (`:217-227`) replays the objective, the slot
        // and every score it already holds.
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::Sidebar,
            Some(&bbb_protocol::JavaString::from("kills")),
        );
        assert_eq!(
            scoreboard.take_pending(),
            vec![
                ClientboundPlayPacket::SetObjective(SetObjective {
                    objective_name: "kills".into(),
                    method: SetObjectiveMethod::Add.into(),
                    parameters: Some(SetObjectiveParameters {
                        display_name: component(
                            text("kills"),
                            bbb_protocol::content_registry::ContentRegistry::vanilla(),
                        ),
                        render_type: ObjectiveRenderType::Integer,
                        number_format: None,
                    }),
                }),
                ClientboundPlayPacket::SetDisplayObjective(SetDisplayObjective {
                    slot: ScoreboardDisplaySlot::Sidebar,
                    objective_name: Some("kills".into()),
                }),
                ClientboundPlayPacket::SetScore(SetScore {
                    owner: "bob".into(),
                    objective_name: "kills".into(),
                    score: 4,
                    display: None,
                    number_format: None,
                }),
            ]
        );

        // Now that it is tracked, the next write goes out on its own.
        scoreboard.set_score("bob", None, "kills", 5);
        assert_eq!(
            scoreboard.take_pending(),
            vec![ClientboundPlayPacket::SetScore(SetScore {
                owner: "bob".into(),
                objective_name: "kills".into(),
                score: 5,
                display: None,
                number_format: None,
            })]
        );

        // `ScoreAccess.set` only sends when something changed
        // (`Scoreboard.java:106-108`).
        scoreboard.set_score("bob", None, "kills", 5);
        assert_eq!(scoreboard.take_pending(), Vec::new());
    }

    /// `setDisplayObjective` (`ServerScoreboard.java:94-114`): the slot the old
    /// objective is leaving stops tracking it only when no other slot still
    /// shows it (`:97-103`).
    #[test]
    fn an_objective_stops_being_tracked_only_when_its_last_slot_drops_it() {
        let mut scoreboard = scoreboard(&["kills"]);
        for slot in [ScoreboardDisplaySlot::Sidebar, ScoreboardDisplaySlot::List] {
            scoreboard.set_display_objective(slot, Some(&bbb_protocol::JavaString::from("kills")));
        }
        scoreboard.take_pending();

        scoreboard.set_display_objective(ScoreboardDisplaySlot::Sidebar, None);
        // The list slot still shows it, so the clients are told about the slot
        // rather than about the objective going away (`:98-99`).
        assert_eq!(
            scoreboard.take_pending(),
            vec![ClientboundPlayPacket::SetDisplayObjective(
                SetDisplayObjective {
                    slot: ScoreboardDisplaySlot::Sidebar,
                    objective_name: None,
                }
            )]
        );

        scoreboard.set_display_objective(ScoreboardDisplaySlot::List, None);
        // `stopTrackingObjective` (`:242-252`) over `getStopTrackingPackets`
        // (`:229-240`), which names the objective being removed.
        assert_eq!(
            scoreboard.take_pending(),
            vec![ClientboundPlayPacket::SetObjective(SetObjective {
                objective_name: "kills".into(),
                method: SetObjectiveMethod::Remove.into(),
                parameters: None,
            })]
        );
        scoreboard.set_score("bob", None, "kills", 1);
        assert_eq!(scoreboard.take_pending(), Vec::new());
    }

    /// `onPlayerRemoved` (`ServerScoreboard.java:77-81`) broadcasts whether or
    /// not anything tracks the owner, while `onPlayerScoreRemoved` (`:84-91`)
    /// is gated on the objective being tracked.
    #[test]
    fn the_two_reset_callbacks_differ_on_whether_tracking_gates_them() {
        let mut scoreboard = scoreboard(&["kills", "deaths"]);
        scoreboard.set_score("bob", None, "kills", 1);
        scoreboard.set_score("bob", None, "deaths", 2);
        scoreboard.take_pending();

        // Untracked, and still broadcast: this is the owner-wide reset.
        scoreboard.reset_all_scores("bob");
        assert_eq!(
            scoreboard.take_pending(),
            vec![ClientboundPlayPacket::ResetScore(ResetScore {
                owner: "bob".into(),
                objective_name: None,
            })]
        );

        scoreboard.set_score("bob", None, "kills", 1);
        scoreboard.set_score("bob", None, "deaths", 2);
        scoreboard.take_pending();
        // Untracked single reset: nothing goes out.
        scoreboard.reset_score("bob", "kills");
        assert_eq!(scoreboard.take_pending(), Vec::new());
        // The last score takes the owner with it, which reports the owner-wide
        // removal instead (`Scoreboard.java:200-204`).
        scoreboard.reset_score("bob", "deaths");
        assert_eq!(
            scoreboard.take_pending(),
            vec![ClientboundPlayPacket::ResetScore(ResetScore {
                owner: "bob".into(),
                objective_name: None,
            })]
        );
    }

    /// `PlayerList.updateEntireScoreboard` (`PlayerList.java:220-237`): every
    /// team, then one start-tracking run per displayed objective, and an
    /// objective on two slots is replayed once (`:229-234`).
    #[test]
    fn a_join_replays_the_teams_and_each_displayed_objective_once() {
        let mut scoreboard = scoreboard(&["kills"]);
        scoreboard.board.add_team("red");
        scoreboard.board.add_player_to_team("bob", "red");
        for slot in [ScoreboardDisplaySlot::Sidebar, ScoreboardDisplaySlot::List] {
            scoreboard.set_display_objective(slot, Some(&bbb_protocol::JavaString::from("kills")));
        }
        scoreboard.take_pending();

        let packets = scoreboard.join_packets();
        assert_eq!(packets.len(), 4);
        assert!(matches!(
            &packets[0],
            ClientboundPlayPacket::SetPlayerTeam(team)
                if team.name == "red" && team.players == ["bob"]
        ));
        assert!(matches!(
            &packets[1],
            ClientboundPlayPacket::SetObjective(objective)
                if objective.objective_name == "kills"
        ));
        // Both slots showing it, in the `DisplaySlot.values()` order the loop
        // walks (`ServerScoreboard.java:200-204`).
        assert_eq!(
            packets[2..],
            [
                ClientboundPlayPacket::SetDisplayObjective(SetDisplayObjective {
                    slot: ScoreboardDisplaySlot::List,
                    objective_name: Some("kills".into()),
                }),
                ClientboundPlayPacket::SetDisplayObjective(SetDisplayObjective {
                    slot: ScoreboardDisplaySlot::Sidebar,
                    objective_name: Some("kills".into()),
                }),
            ]
        );
    }

    /// `ServerScoreboard.load` (`:36-44`) runs before any session is placed, so
    /// the display-slot writes it makes start tracking without queuing anything
    /// for a client that is not there.
    #[test]
    fn a_restore_tracks_its_display_slots_and_queues_nothing() {
        let packed = Java26ScoreboardPacked {
            objectives: vec![bbb_server_persistence::Java26ScoreboardObjective {
                name: "kills".into(),
                criteria: "dummy".to_string(),
                display_name: text("kills"),
                render_type: ObjectiveRenderType::Integer,
                display_auto_update: false,
                format: None,
            }],
            scores: Vec::new(),
            display_slots: vec![(ScoreboardDisplaySlot::Sidebar, "kills".into())],
            teams: Vec::new(),
        };
        let mut scoreboard = ServerScoreboard::restore(&packed);
        assert_eq!(scoreboard.take_pending(), Vec::new());
        // Tracked all the same, so the next write does reach a client.
        scoreboard.set_score("bob", None, "kills", 1);
        assert_eq!(scoreboard.take_pending().len(), 1);
    }
}
