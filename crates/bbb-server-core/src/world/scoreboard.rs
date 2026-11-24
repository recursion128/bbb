//! The live `Scoreboard` (`Scoreboard.java:30-38`).
//!
//! The store this restores from and packs back into is
//! [`bbb_server_persistence::Java26ScoreboardPacked`], which is already the
//! whole of `ScoreboardSaveData.Packed` (`ScoreboardSaveData.java:40-52`); what
//! is added here is the live half `Scoreboard` owns: the by-criteria index
//! `forAllObjectives` walks, the per-owner score map, the display slots and the
//! two team indices.
//!
//! **Divergence.** Vanilla's `objectivesByName`, `playerScores` and
//! `teamsByName` are `Object2ObjectOpenHashMap`s (`:33`, `:35`, `:37`) and
//! `packObjectives`/`packPlayerScores`/`packPlayerTeams` (`:396-397`,
//! `:349-364`, `:375-377`) write them out in that map's bucket order, which is
//! `java.util.HashMap`'s order over `String.hashCode` and is not derivable from
//! the vanilla source this workspace verifies against. Insertion order is kept
//! here instead, the way [`crate::commands::CommandPlayer::tags`] keeps
//! it. Every single-entry document is identical either way; one with two or
//! more can list them in a different order than the official server would.
//!
//! Nothing here reaches a client. This is `Scoreboard` alone; the fan-out its
//! `on*` hooks stand for is [`super::server_scoreboard::ServerScoreboard`],
//! which is the only thing that may mutate one of these.

use crate::entity::ScoreboardTeams;
use bbb_protocol::JavaString;

pub(crate) trait ScoreboardName {
    fn matches(&self, name: &JavaString) -> bool;
    fn to_java(&self) -> JavaString;
    fn with_units<T>(&self, apply: impl FnOnce(&[u16]) -> T) -> T;
}

impl ScoreboardName for JavaString {
    fn matches(&self, name: &JavaString) -> bool {
        self == name
    }
    fn to_java(&self) -> JavaString {
        self.clone()
    }
    fn with_units<T>(&self, apply: impl FnOnce(&[u16]) -> T) -> T {
        apply(self.units())
    }
}
impl ScoreboardName for str {
    fn matches(&self, name: &JavaString) -> bool {
        name == self
    }
    fn to_java(&self) -> JavaString {
        self.into()
    }
    fn with_units<T>(&self, apply: impl FnOnce(&[u16]) -> T) -> T {
        JavaString::with_utf16(self, apply)
    }
}
impl ScoreboardName for String {
    fn matches(&self, name: &JavaString) -> bool {
        name == self
    }
    fn to_java(&self) -> JavaString {
        self.into()
    }
    fn with_units<T>(&self, apply: impl FnOnce(&[u16]) -> T) -> T {
        JavaString::with_utf16(self, apply)
    }
}

use crate::commands::types::CommandDamageEntityRef;
use crate::entity::Entity;
use uuid::Uuid;

use bbb_facts::scoreboard_team_options::{TeamCollisionRule, TeamVisibility};
use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::objective_render_types::ObjectiveRenderType;
use bbb_protocol::scoreboard_display_slots::ScoreboardDisplaySlot;
use bbb_protocol::NbtSummaryValue;
use bbb_server_persistence::{
    Java26ScoreboardNumberFormat, Java26ScoreboardObjective, Java26ScoreboardPacked,
    Java26ScoreboardPackedScore, Java26ScoreboardTeam,
};

/// The two properties `ObjectiveCriteria` carries beside its name
/// (`ObjectiveCriteria.java:70-72`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CriteriaFacts {
    /// `isReadOnly` (`:121-123`), which `getOrCreatePlayerScore` checks before
    /// it lets a score be written (`Scoreboard.java:76`).
    pub read_only: bool,
    /// `getDefaultRenderType` (`:125-127`).
    pub render_type: ObjectiveRenderType,
}

/// The six criteria `registerCustom` gives a read-only flag
/// (`ObjectiveCriteria.java:28-33`), with the render type each was registered
/// under. Every other built-in takes the two-argument `registerCustom`
/// (`:80-82`), and a stat criterion reaches `ObjectiveCriteria(String)`
/// through `Stat`'s constructor (`Stat.java:20-25`, `ObjectiveCriteria.java:84-86`),
/// so both are writable and integer-rendered.
const READ_ONLY_CRITERIA: &[(&str, ObjectiveRenderType)] = &[
    ("health", ObjectiveRenderType::Hearts),
    ("food", ObjectiveRenderType::Integer),
    ("air", ObjectiveRenderType::Integer),
    ("armor", ObjectiveRenderType::Integer),
    ("xp", ObjectiveRenderType::Integer),
    ("level", ObjectiveRenderType::Integer),
];

/// `ObjectiveCriteria.byName` (`:99-111`) reduced to the two properties the
/// live scoreboard reads. A name no criterion carries is refused at the
/// document boundary, so anything reaching here that is not built in is a stat.
pub(crate) fn criteria_facts(name: &str) -> CriteriaFacts {
    READ_ONLY_CRITERIA
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map_or(
            CriteriaFacts {
                read_only: false,
                render_type: ObjectiveRenderType::Integer,
            },
            |(_, render_type)| CriteriaFacts {
                read_only: true,
                render_type: *render_type,
            },
        )
}

/// `Objective` (`Objective.java:16-24`) minus the two derived fields: the
/// scoreboard back-reference and `formattedDisplayName`, which
/// `createFormattedDisplayName` rebuilds from `displayName` on every write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Objective {
    pub name: JavaString,
    pub criteria: String,
    pub display_name: NbtSummaryValue,
    pub render_type: ObjectiveRenderType,
    pub display_auto_update: bool,
    pub number_format: Option<Java26ScoreboardNumberFormat>,
}

/// `Score` (`Score.java:13-17`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Score {
    pub value: i32,
    pub locked: bool,
    pub display: Option<NbtSummaryValue>,
    pub number_format: Option<Java26ScoreboardNumberFormat>,
}

impl Default for Score {
    /// The field initialisers (`Score.java:14-17`). `locked` starts **true**
    /// where `Score.Packed`'s codec defaults the absent `Locked` field to false
    /// (`:72`), so a freshly tracked score and a restored one with no `Locked`
    /// written disagree -- which is the whole reason `/trigger` needs the flag.
    fn default() -> Self {
        Self {
            value: 0,
            locked: true,
            display: None,
            number_format: None,
        }
    }
}

/// What `resetSinglePlayerScore` (`Scoreboard.java:196-209`) did, which is what
/// picks between `onPlayerRemoved` and `onPlayerScoreRemoved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScoreResetOutcome {
    Nothing,
    ScoreRemoved,
    OwnerRemoved,
}

/// `PlayerTeam` (`PlayerTeam.java:20-35`) minus `displayNameStyle`, which the
/// constructor derives from the name (`:41`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerTeam {
    pub name: JavaString,
    pub display_name: NbtSummaryValue,
    pub player_prefix: Option<NbtSummaryValue>,
    pub player_suffix: Option<NbtSummaryValue>,
    pub allow_friendly_fire: bool,
    pub see_friendly_invisibles: bool,
    pub name_tag_visibility: TeamVisibility,
    pub death_message_visibility: TeamVisibility,
    pub color: ChatFormatting,
    pub collision_rule: TeamCollisionRule,
}

/// What one `addPlayerToTeam` did (`Scoreboard.java:272-279`), which is two
/// facts rather than one: the team the player was taken off first and whether
/// the new team's member set actually grew.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TeamJoin {
    pub left: Option<JavaString>,
    pub added: bool,
}

impl PlayerTeam {
    /// `new PlayerTeam(scoreboard, name)` (`PlayerTeam.java:37-42`) over the
    /// field initialisers at `:26-34`.
    pub(crate) fn new(name: &(impl ScoreboardName + ?Sized)) -> Self {
        Self {
            name: name.to_java(),
            display_name: NbtSummaryValue::String(name.to_java()),
            player_prefix: None,
            player_suffix: None,
            allow_friendly_fire: true,
            see_friendly_invisibles: true,
            name_tag_visibility: TeamVisibility::Always,
            death_message_visibility: TeamVisibility::Always,
            color: ChatFormatting::Reset,
            collision_rule: TeamCollisionRule::Always,
        }
    }
}

fn is_stat_criteria(criteria: &str) -> bool {
    criteria.contains(':')
}

fn is_ticked_criteria(criteria: &str) -> bool {
    READ_ONLY_CRITERIA
        .iter()
        .any(|(candidate, _)| *candidate == criteria)
}

/// `Scoreboard` (`Scoreboard.java:30-38`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Scoreboard {
    /// `objectivesByName` (`:33`), in insertion order.
    objectives: Vec<Objective>,
    /// `objectivesByCriteria` (`:34`): criteria name -> indices into `objectives` in add order.
    objectives_by_criteria: rustc_hash::FxHashMap<String, Vec<usize>>,
    /// Count of objectives backed by a stat criterion (`criteria.contains(':')`).
    stat_objective_count: usize,
    /// Count of objectives backed by one of the six per-tick player criteria.
    ticked_criteria_count: usize,
    /// `playerScores` (`:35`) flattened: owner, then objective name.
    player_scores: Vec<(JavaString, Vec<(JavaString, Score)>)>,
    /// O(1) owner-name -> index into `player_scores`, keyed on UTF-16 units.
    player_score_owners: rustc_hash::FxHashMap<Box<[u16]>, usize>,
    /// `displayObjectives` (`:36`), an `EnumMap` and so kept in slot order
    /// rather than in the order the slots were set.
    display_objectives: Vec<(ScoreboardDisplaySlot, JavaString)>,
    /// Team metadata and the canonical raw member index borrowed by entity inputs.
    pub(crate) teams: ScoreboardTeams,
}

impl Scoreboard {
    /// `Entity.getTeam` (Entity.java:2692-2694); tamed pets inherit their
    /// root owner's team (TamableAnimal.java:190-204).
    pub(crate) fn entity_team_name<'e>(
        &self,
        entity: CommandDamageEntityRef<'_>,
        resolve_living: impl Fn(Uuid) -> Option<CommandDamageEntityRef<'e>>,
    ) -> Option<&JavaString> {
        match entity {
            CommandDamageEntityRef::Player(player) => {
                self.players_team(player.name).map(|team| &team.name)
            }
            CommandDamageEntityRef::Stored(stored) => {
                self.stored_entity_team_name(stored.entity, resolve_living)
            }
        }
    }

    pub(crate) fn stored_entity_team_name<'e>(
        &self,
        entity: &Entity,
        resolve_living: impl Fn(Uuid) -> Option<CommandDamageEntityRef<'e>>,
    ) -> Option<&JavaString> {
        let tame_owner = entity
            .tamable_owner()
            .filter(|(tame, _)| *tame)
            .and_then(|(_, owner)| owner);
        self.stored_team_name(entity.uuid(), tame_owner, resolve_living)
    }

    pub(crate) fn stored_team_name<'e>(
        &self,
        entity_uuid: Uuid,
        tame_owner: Option<Uuid>,
        resolve_living: impl Fn(Uuid) -> Option<CommandDamageEntityRef<'e>>,
    ) -> Option<&JavaString> {
        let team_of = |name: &str| self.players_team(name).map(|team| &team.name);
        let mut buffer = Uuid::encode_buffer();
        if let Some(team) = team_of(entity_uuid.hyphenated().encode_lower(&mut buffer)) {
            return Some(team);
        }
        let owner = root_owner(entity_uuid, tame_owner?, &resolve_living)?;
        owner.with_scoreboard_name(team_of)
    }

    /// `getObjective` (`:40-42`).
    pub(crate) fn objective(&self, name: &(impl ScoreboardName + ?Sized)) -> Option<&Objective> {
        self.objectives
            .iter()
            .find(|entry| name.matches(&entry.name))
    }

    /// The same lookup for the setters `Objective` exposes
    /// (`Objective.java:87-110`), each of which writes one field and then tells
    /// the scoreboard.
    pub(crate) fn objective_mut(
        &mut self,
        name: &(impl ScoreboardName + ?Sized),
    ) -> Option<&mut Objective> {
        self.objectives
            .iter_mut()
            .find(|entry| name.matches(&entry.name))
    }

    /// `displayObjectives` (`:36`) as the slot loops walk it.
    pub(crate) fn display_objectives(&self) -> &[(ScoreboardDisplaySlot, JavaString)] {
        &self.display_objectives
    }

    /// `addObjective` (`:44-61`). `None` is the `IllegalArgumentException` at
    /// `:52-54`, which the callers all guard against first.
    pub(crate) fn add_objective(&mut self, objective: Objective) -> Option<&Objective> {
        if self.objective(&objective.name).is_some() {
            return None;
        }
        let index = self.objectives.len();
        if is_stat_criteria(&objective.criteria) {
            self.stat_objective_count += 1;
        }
        if is_ticked_criteria(&objective.criteria) {
            self.ticked_criteria_count += 1;
        }
        self.objectives_by_criteria
            .entry(objective.criteria.clone())
            .or_default()
            .push(index);
        self.objectives.push(objective);
        self.objectives.last()
    }

    fn rebuild_criteria_index(&mut self) {
        self.objectives_by_criteria.clear();
        self.stat_objective_count = 0;
        self.ticked_criteria_count = 0;
        for (index, objective) in self.objectives.iter().enumerate() {
            if is_stat_criteria(&objective.criteria) {
                self.stat_objective_count += 1;
            }
            if is_ticked_criteria(&objective.criteria) {
                self.ticked_criteria_count += 1;
            }
            self.objectives_by_criteria
                .entry(objective.criteria.clone())
                .or_default()
                .push(index);
        }
    }

    /// `setDisplayObjective` (`:237-239`); `None` is the null that clears it.
    pub(crate) fn set_display_objective(
        &mut self,
        slot: ScoreboardDisplaySlot,
        objective: Option<&JavaString>,
    ) {
        self.display_objectives
            .retain(|(existing, _)| existing.id() != slot.id());
        if let Some(name) = objective {
            self.display_objectives.push((slot, name.to_java()));
            self.display_objectives
                .sort_by_key(|(existing, _)| existing.id());
        }
    }

    /// `getOrCreatePlayerScore` (`:71-159`) through `PlayerScores.getOrCreate`
    /// (`PlayerScores.java:18-24`), which files a default [`Score`] the first
    /// time an owner is tracked.
    ///
    /// `force_writable` is the flag `forAllObjectives` passes (`:64`); without
    /// it a read-only criterion refuses the write the way `set` throws at
    /// `:88-90`.
    pub(crate) fn score_mut(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
        force_writable: bool,
    ) -> Option<&mut Score> {
        let criteria = self.objective(objective)?.criteria.clone();
        // `canModify` (`:76`) is decided before the entry is filed, and the
        // entry is filed either way: only `set` consults it (`:88-90`), so a
        // read-only criterion still gains a tracked score from every other
        // `ScoreAccess` write.
        let writable = force_writable || !criteria_facts(&criteria).read_only;
        let (score, _) = self.score_entry_mut(owner, objective)?;
        writable.then_some(score)
    }

    /// `getOrCreatePlayerInfo` and `PlayerScores.getOrCreate`
    /// (`Scoreboard.java:67-69`, `PlayerScores.java:18-24`) on their own,
    /// answering whether this call is the one that filed the score --
    /// `requiresSync` (`Scoreboard.java:78-79`), which is what makes a first
    /// write reach the clients even when it changes nothing.
    fn owner_score_index(&self, owner: &(impl ScoreboardName + ?Sized)) -> Option<usize> {
        owner.with_units(|units| self.player_score_owners.get(units).copied())
    }

    fn remove_owner_at(&mut self, index: usize) {
        let (removed, _) = self.player_scores.remove(index);
        self.player_score_owners.remove(removed.units());
        for (idx, (name, _)) in self.player_scores.iter().enumerate().skip(index) {
            if let Some(slot) = self.player_score_owners.get_mut(name.units()) {
                *slot = idx;
            }
        }
    }

    pub(crate) fn score_entry_mut(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
    ) -> Option<(&mut Score, bool)> {
        self.objective(objective)?;
        let owner_index = match self.owner_score_index(owner) {
            Some(index) => index,
            None => {
                let java = owner.to_java();
                let index = self.player_scores.len();
                self.player_score_owners.insert(java.units().into(), index);
                self.player_scores.push((java, Vec::new()));
                index
            }
        };
        let scores = &mut self.player_scores[owner_index].1;
        let (score_index, created) =
            match scores.iter().position(|(name, _)| objective.matches(name)) {
                Some(index) => (index, false),
                None => {
                    scores.push((objective.to_java(), Score::default()));
                    (scores.len() - 1, true)
                }
            };
        Some((&mut scores[score_index].1, created))
    }

    fn team_index(&self, name: &(impl ScoreboardName + ?Sized)) -> Option<usize> {
        self.teams()
            .iter()
            .position(|team| name.matches(&team.name))
    }

    /// `getPlayerTeam` (`:245-247`).
    pub(crate) fn team(&self, name: &(impl ScoreboardName + ?Sized)) -> Option<&PlayerTeam> {
        self.team_index(name).map(|index| &self.teams.teams[index])
    }

    pub(crate) fn team_mut(
        &mut self,
        name: &(impl ScoreboardName + ?Sized),
    ) -> Option<&mut PlayerTeam> {
        let index = self.team_index(name)?;
        Some(&mut self.teams.teams[index])
    }

    pub(crate) fn teams(&self) -> &[PlayerTeam] {
        &self.teams.teams
    }

    pub(crate) fn team_members(
        &self,
        name: &(impl ScoreboardName + ?Sized),
    ) -> impl Iterator<Item = &JavaString> {
        let index = self.team_index(name);
        self.teams
            .members
            .iter()
            .filter(move |(_, team)| Some(**team as usize) == index)
            .map(|(member, _)| member)
    }

    /// `addPlayerTeam` (`:249-260`), whose existing-team arm warns and answers
    /// with the team that is already there rather than replacing it.
    pub(crate) fn add_team(&mut self, name: &(impl ScoreboardName + ?Sized)) -> bool {
        if self.team(name).is_some() {
            return false;
        }
        self.teams.teams.push(PlayerTeam::new(name));
        true
    }

    /// `addPlayerToTeam` (`:272-279`), which leaves the team the player was on
    /// before it joins the new one.
    pub(crate) fn add_player_to_team(
        &mut self,
        player: &(impl ScoreboardName + ?Sized),
        team: &(impl ScoreboardName + ?Sized),
    ) -> Option<TeamJoin> {
        let index = self.team_index(team)? as u32;
        let left = self.remove_player_from_team(player);
        let added = self.teams.members.insert(player.to_java(), index).is_none();
        Some(TeamJoin { left, added })
    }

    /// `removePlayerFromTeam(String)` (`:281-289`), answering with the team
    /// `getPlayersTeam` found rather than with a bare flag: the fan-out names
    /// that team in the packet it sends.
    pub(crate) fn remove_player_from_team(
        &mut self,
        player: &(impl ScoreboardName + ?Sized),
    ) -> Option<JavaString> {
        let index = player.with_units(|player| self.teams.members.remove(player))?;
        Some(self.teams.teams[index as usize].name.clone())
    }

    /// `removePlayerFromTeam(String, PlayerTeam)` (`:291-298`). The
    /// `IllegalStateException` guard (`:292-294`) has no expression here: both
    /// callers pass the team the player is already on.
    pub(crate) fn remove_player_from_named_team(
        &mut self,
        player: &(impl ScoreboardName + ?Sized),
        team: &(impl ScoreboardName + ?Sized),
    ) -> bool {
        player.with_units(|player| {
            let Some(index) = self.teams.members.get(player) else {
                return false;
            };
            if !team.matches(&self.teams.teams[*index as usize].name) {
                return false;
            }
            self.teams.members.remove(player).is_some()
        })
    }

    /// `removePlayerTeam` (`:262-270`), which also clears the by-player index
    /// for every member. The scores of those members are untouched.
    pub(crate) fn remove_team(&mut self, name: &(impl ScoreboardName + ?Sized)) -> bool {
        let Some(index) = self.team_index(name) else {
            return false;
        };
        let data = &mut self.teams;
        data.teams.remove(index);
        data.members.retain(|_, team| {
            if *team == index as u32 {
                return false;
            }
            if *team > index as u32 {
                *team -= 1;
            }
            true
        });
        true
    }

    /// `loadObjective`, `loadPlayerScore`, `loadPlayerTeam` and the display-slot
    /// map, in the order `ScoreboardSaveData` hands them over
    /// (`Scoreboard.java:400-409`, `:366-373`, `:379-394`,
    /// `ScoreboardSaveData.java:40-52`).
    pub(crate) fn restore(packed: &Java26ScoreboardPacked) -> Self {
        let mut scoreboard = Self::default();
        for objective in &packed.objectives {
            scoreboard.add_objective(Objective {
                name: objective.name.clone(),
                criteria: objective.criteria.clone(),
                display_name: objective.display_name.clone(),
                render_type: objective.render_type,
                display_auto_update: objective.display_auto_update,
                number_format: objective.format.clone(),
            });
        }
        for score in &packed.scores {
            // `setScore` (`PlayerScores.java:40-42`) files the packed instance
            // whole, so this is not the writable path and a read-only criterion
            // is restored like any other. `score_mut`'s own objective lookup is
            // `loadPlayerScore`'s null check (`:367-369`), which drops a score
            // whose objective the document never declared.
            let restored = Score {
                value: score.score,
                locked: score.locked,
                display: score.display.clone(),
                number_format: score.format.clone(),
            };
            if let Some(slot) = scoreboard.score_mut(&score.owner, &score.objective, true) {
                *slot = restored;
            }
        }
        for (slot, objective) in &packed.display_slots {
            scoreboard.set_display_objective(*slot, Some(objective));
        }
        for team in &packed.teams {
            scoreboard.add_team(&team.name);
            let entry = scoreboard
                .team_mut(&team.name)
                .expect("the team was just added");
            // `loadPlayerTeam` (`:379-394`) only writes the display name and
            // the colour when the packed value carries one, leaving the
            // constructor's default in place otherwise.
            if let Some(display_name) = &team.display_name {
                entry.display_name = display_name.clone();
            }
            if let Some(color) = team.color {
                entry.color = color;
            }
            entry.allow_friendly_fire = team.allow_friendly_fire;
            entry.see_friendly_invisibles = team.see_friendly_invisibles;
            entry.player_prefix = team.member_name_prefix.clone();
            entry.player_suffix = team.member_name_suffix.clone();
            entry.name_tag_visibility = team.name_tag_visibility;
            entry.death_message_visibility = team.death_message_visibility;
            entry.collision_rule = team.collision_rule;
            for player in &team.players {
                scoreboard.add_player_to_team(player, &team.name);
            }
        }
        scoreboard
    }

    /// `packObjectives`, `packPlayerScores`, `packDisplaySlots` and
    /// `packPlayerTeams` (`Scoreboard.java:396-398`, `:349-364`, `:411-422`,
    /// `:375-377`).
    pub(crate) fn pack(&self) -> Java26ScoreboardPacked {
        let mut packed = Java26ScoreboardPacked {
            objectives: self
                .objectives
                .iter()
                .map(|objective| Java26ScoreboardObjective {
                    name: objective.name.clone(),
                    criteria: objective.criteria.clone(),
                    display_name: objective.display_name.clone(),
                    render_type: objective.render_type,
                    display_auto_update: objective.display_auto_update,
                    format: objective.number_format.clone(),
                })
                .collect(),
            scores: self
                .player_scores
                .iter()
                .flat_map(|(owner, scores)| {
                    scores
                        .iter()
                        .map(move |(objective, score)| Java26ScoreboardPackedScore {
                            owner: owner.clone(),
                            objective: objective.clone(),
                            score: score.value,
                            locked: score.locked,
                            display: score.display.clone(),
                            format: score.number_format.clone(),
                        })
                })
                .collect(),
            display_slots: self.display_objectives.clone(),
            teams: self
                .teams()
                .iter()
                .map(|team| Java26ScoreboardTeam {
                    name: team.name.clone(),
                    // `pack` (`PlayerTeam.java:44-58`) always writes the display
                    // name and writes the colour only when it is not `RESET`
                    // (`:48`).
                    display_name: Some(team.display_name.clone()),
                    color: (team.color != ChatFormatting::Reset).then_some(team.color),
                    allow_friendly_fire: team.allow_friendly_fire,
                    see_friendly_invisibles: team.see_friendly_invisibles,
                    member_name_prefix: team.player_prefix.clone(),
                    member_name_suffix: team.player_suffix.clone(),
                    name_tag_visibility: team.name_tag_visibility,
                    death_message_visibility: team.death_message_visibility,
                    collision_rule: team.collision_rule,
                    players: Vec::new(),
                })
                .collect(),
        };
        for (member, team) in &self.teams.members {
            packed.teams[*team as usize].players.push(member.clone());
        }
        packed
    }
}

impl Scoreboard {
    pub(crate) fn objectives(&self) -> &[Objective] {
        &self.objectives
    }

    pub(crate) fn objective_indices_for_criteria(&self, criteria: &str) -> &[usize] {
        self.objectives_by_criteria
            .get(criteria)
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn objectives_for_criteria(
        &self,
        criteria: &str,
    ) -> impl Iterator<Item = &Objective> + '_ {
        self.objective_indices_for_criteria(criteria)
            .iter()
            .map(|&index| &self.objectives[index])
    }

    pub(crate) fn has_stat_objectives(&self) -> bool {
        self.stat_objective_count > 0
    }

    pub(crate) fn has_ticked_criteria_objectives(&self) -> bool {
        self.ticked_criteria_count > 0
    }

    /// `removeObjective` (`:216-235`): the name index, then every display slot
    /// showing it, then the criteria index, then every owner's score.
    pub(crate) fn remove_objective(&mut self, name: &(impl ScoreboardName + ?Sized)) -> bool {
        let Some(index) = self
            .objectives
            .iter()
            .position(|entry| name.matches(&entry.name))
        else {
            return false;
        };
        self.objectives.remove(index);
        self.rebuild_criteria_index();
        self.display_objectives
            .retain(|(_, displayed)| !name.matches(displayed));
        for (_, scores) in &mut self.player_scores {
            scores.retain(|(objective, _)| !name.matches(objective));
        }
        true
    }

    /// `getDisplayObjective` (`:241-243`).
    pub(crate) fn display_objective(&self, slot: ScoreboardDisplaySlot) -> Option<&JavaString> {
        self.display_objectives
            .iter()
            .find(|(existing, _)| existing.id() == slot.id())
            .map(|(_, objective)| objective)
    }

    /// `getPlayerScoreInfo` (`:161-164`): the score an owner already has, which
    /// is `null` rather than a fresh one when nothing tracks it yet.
    pub(crate) fn score(
        &self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
    ) -> Option<&Score> {
        let index = self.owner_score_index(owner)?;
        self.player_scores[index]
            .1
            .iter()
            .find(|(name, _)| objective.matches(name))
            .map(|(_, score)| score)
    }

    /// `resetAllPlayerScores` (`:189-194`).
    pub(crate) fn reset_all_scores(&mut self, owner: &(impl ScoreboardName + ?Sized)) -> bool {
        let Some(index) = self.owner_score_index(owner) else {
            return false;
        };
        self.remove_owner_at(index);
        true
    }

    /// `resetSinglePlayerScore` (`:196-209`), including the arm that drops the
    /// owner entirely once its last score is gone (`:200-204`) and reports the
    /// owner-wide removal rather than the single one.
    pub(crate) fn reset_score(
        &mut self,
        owner: &(impl ScoreboardName + ?Sized),
        objective: &(impl ScoreboardName + ?Sized),
    ) -> ScoreResetOutcome {
        let Some(index) = self.owner_score_index(owner) else {
            return ScoreResetOutcome::Nothing;
        };
        let scores = &mut self.player_scores[index].1;
        let before = scores.len();
        scores.retain(|(name, _)| !objective.matches(name));
        let removed = scores.len() != before;
        if scores.is_empty() {
            self.remove_owner_at(index);
            return ScoreResetOutcome::OwnerRemoved;
        }
        if removed {
            ScoreResetOutcome::ScoreRemoved
        } else {
            ScoreResetOutcome::Nothing
        }
    }

    /// `getTrackedPlayers` (`:185-187`).
    pub(crate) fn tracked_owners(&self) -> impl Iterator<Item = &JavaString> {
        self.player_scores.iter().map(|(owner, _)| owner)
    }

    /// `listPlayerScores(ScoreHolder)` (`:211-214`) through
    /// `PlayerScores.listScores` (`PlayerScores.java:34-38`), which walks the
    /// owner's own map rather than the objective list.
    pub(crate) fn scores_of(
        &self,
        owner: &(impl ScoreboardName + ?Sized),
    ) -> Vec<(&JavaString, &Score)> {
        let Some(index) = self.owner_score_index(owner) else {
            return Vec::new();
        };
        self.player_scores[index]
            .1
            .iter()
            .map(|(objective, score)| (objective, score))
            .collect()
    }

    /// `listPlayerScores(Objective)` (`:166-175`).
    pub(crate) fn scores_for(
        &self,
        objective: &(impl ScoreboardName + ?Sized),
    ) -> Vec<(&JavaString, &Score)> {
        self.player_scores
            .iter()
            .filter_map(|(owner, scores)| {
                scores
                    .iter()
                    .find(|(name, _)| objective.matches(name))
                    .map(|(_, score)| (owner, score))
            })
            .collect()
    }

    /// `getPlayersTeam` (`:308-310`) over the raw member index.
    pub(crate) fn players_team(
        &self,
        player: &(impl ScoreboardName + ?Sized),
    ) -> Option<&PlayerTeam> {
        if self.teams.is_empty() {
            return None;
        }
        player
            .with_units(|player| self.teams.members.get(player))
            .map(|team| &self.teams.teams[*team as usize])
    }

    /// `Player.canHarmPlayer`'s team arm (`Player.java:727-735`) between two
    /// scoreboard names.
    pub(crate) fn team_allows_player_harm(
        &self,
        attacker: &(impl ScoreboardName + ?Sized),
        target: &(impl ScoreboardName + ?Sized),
    ) -> bool {
        let Some(team) = self.players_team(attacker) else {
            return true;
        };
        self.players_team(target)
            .is_none_or(|other| other.name != team.name)
            || team.allow_friendly_fire
    }
}

/// `OwnableEntity.getRootOwner` (OwnableEntity.java:17-34).
fn root_owner<'a>(
    start_uuid: Uuid,
    first_owner: Uuid,
    resolve_living: &impl Fn(Uuid) -> Option<CommandDamageEntityRef<'a>>,
) -> Option<CommandDamageEntityRef<'a>> {
    let mut seen = vec![start_uuid];
    let mut owner = resolve_living(first_owner)?;
    loop {
        let CommandDamageEntityRef::Stored(ownable) = owner else {
            return Some(owner);
        };
        let Some(reference) = ownable.entity.ownable_owner() else {
            return Some(owner);
        };
        let next = reference.and_then(resolve_living);
        if next.is_some_and(|next| seen.contains(&next.uuid())) {
            return None;
        }
        seen.push(owner.uuid());
        owner = next?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> NbtSummaryValue {
        NbtSummaryValue::string(value)
    }

    fn objective(name: &str, criteria: &str) -> Objective {
        Objective {
            name: name.into(),
            criteria: criteria.to_owned(),
            display_name: text(name),
            render_type: criteria_facts(criteria).render_type,
            display_auto_update: false,
            number_format: None,
        }
    }

    /// `registerCustom` gives exactly six criteria the read-only flag
    /// (`ObjectiveCriteria.java:28-33`) and only `health` the hearts render
    /// type (`:28`); every other built-in takes the two-argument overload
    /// (`:80-82`) and a stat reaches `ObjectiveCriteria(String)` through
    /// `Stat`'s constructor (`Stat.java:20-25`).
    #[test]
    fn only_the_six_registered_read_only_criteria_refuse_a_write() {
        for name in ["health", "food", "air", "armor", "xp", "level"] {
            assert!(criteria_facts(name).read_only, "{name} is read-only");
        }
        for name in [
            "dummy",
            "trigger",
            "deathCount",
            "playerKillCount",
            "totalKillCount",
            "teamkill.red",
            "killedByTeam.blue",
            "minecraft.custom:minecraft.jump",
        ] {
            assert!(!criteria_facts(name).read_only, "{name} is writable");
            assert_eq!(
                criteria_facts(name).render_type,
                ObjectiveRenderType::Integer
            );
        }
        assert_eq!(
            criteria_facts("health").render_type,
            ObjectiveRenderType::Hearts
        );
    }

    /// `Score`'s field initialiser is `locked = true` (`Score.java:15`) while
    /// `Score.Packed`'s codec defaults the absent `Locked` field to false
    /// (`:72`), so a newly tracked score and a restored one disagree.
    #[test]
    fn a_newly_tracked_score_is_locked_where_a_restored_one_need_not_be() {
        let mut scoreboard = Scoreboard::default();
        scoreboard.add_objective(objective("kills", "dummy"));
        assert!(
            scoreboard
                .score_mut("alice", "kills", false)
                .unwrap()
                .locked
        );

        let restored = Scoreboard::restore(&Java26ScoreboardPacked {
            objectives: vec![Java26ScoreboardObjective {
                name: "kills".into(),
                criteria: "dummy".to_owned(),
                display_name: text("kills"),
                render_type: ObjectiveRenderType::Integer,
                display_auto_update: false,
                format: None,
            }],
            scores: vec![Java26ScoreboardPackedScore {
                owner: "alice".into(),
                objective: "kills".into(),
                score: 3,
                locked: false,
                display: None,
                format: None,
            }],
            display_slots: Vec::new(),
            teams: Vec::new(),
        });
        let score = restored.score("alice", "kills").expect("restored");
        assert!(!score.locked);
        assert_eq!(score.value, 3);
    }

    /// `addObjective` throws for a name already taken (`Scoreboard.java:52-54`).
    #[test]
    fn a_second_objective_under_a_taken_name_is_refused() {
        let mut scoreboard = Scoreboard::default();
        assert!(scoreboard
            .add_objective(objective("kills", "dummy"))
            .is_some());
        assert!(scoreboard
            .add_objective(objective("kills", "deathCount"))
            .is_none());
        assert_eq!(scoreboard.objectives().len(), 1);
        assert_eq!(scoreboard.objective("kills").unwrap().criteria, "dummy");
    }

    /// `removeObjective` (`:216-235`) takes the objective off every display
    /// slot showing it and out of every owner's score map.
    #[test]
    fn removing_an_objective_clears_its_display_slot_and_every_score() {
        let mut scoreboard = Scoreboard::default();
        scoreboard.add_objective(objective("kills", "dummy"));
        scoreboard.add_objective(objective("deaths", "dummy"));
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::Sidebar,
            Some(&bbb_protocol::JavaString::from("kills")),
        );
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::List,
            Some(&bbb_protocol::JavaString::from("deaths")),
        );
        scoreboard.score_mut("alice", "kills", false).unwrap().value = 4;
        scoreboard
            .score_mut("alice", "deaths", false)
            .unwrap()
            .value = 1;

        assert!(scoreboard.remove_objective("kills"));
        assert!(scoreboard.objective("kills").is_none());
        assert_eq!(
            scoreboard.display_objective(ScoreboardDisplaySlot::Sidebar),
            None
        );
        assert_eq!(
            scoreboard.display_objective(ScoreboardDisplaySlot::List),
            Some(&bbb_protocol::JavaString::from("deaths"))
        );
        assert!(scoreboard.score("alice", "kills").is_none());
        assert_eq!(scoreboard.score("alice", "deaths").unwrap().value, 1);
        // A name no objective carries is not a removal.
        assert!(!scoreboard.remove_objective("kills"));
    }

    /// `getOrCreatePlayerScore`'s `canModify` (`Scoreboard.java:76`) is
    /// `forceWritable || !criteria.isReadOnly()`, and `forAllObjectives`
    /// (`:63-65`) is the caller that passes the flag.
    #[test]
    fn a_read_only_criterion_is_writable_only_through_the_criteria_walk() {
        let mut scoreboard = Scoreboard::default();
        scoreboard.add_objective(objective("hp", "health"));
        assert!(scoreboard.score_mut("alice", "hp", false).is_none());
        assert!(scoreboard.score_mut("alice", "hp", true).is_some());
        // An objective no name carries has no score either way.
        assert!(scoreboard.score_mut("alice", "missing", true).is_none());
    }

    /// `resetSinglePlayerScore` (`:196-209`) drops the owner entirely once its
    /// last score is gone (`:200-204`), which is what takes it out of
    /// `getTrackedPlayers` (`:185-187`).
    #[test]
    fn an_owner_leaves_the_tracked_set_with_its_last_score() {
        let mut scoreboard = Scoreboard::default();
        scoreboard.add_objective(objective("kills", "dummy"));
        scoreboard.add_objective(objective("deaths", "dummy"));
        scoreboard.score_mut("alice", "kills", false).unwrap().value = 4;
        scoreboard
            .score_mut("alice", "deaths", false)
            .unwrap()
            .value = 1;
        scoreboard.score_mut("bob", "kills", false).unwrap().value = 9;

        assert_eq!(
            scoreboard.reset_score("alice", "kills"),
            ScoreResetOutcome::ScoreRemoved
        );
        assert_eq!(
            scoreboard.tracked_owners().collect::<Vec<_>>(),
            ["alice", "bob"]
        );
        // The last score takes the owner with it, which is the arm that
        // reports the owner-wide removal instead (`:200-204`).
        assert_eq!(
            scoreboard.reset_score("alice", "deaths"),
            ScoreResetOutcome::OwnerRemoved
        );
        assert_eq!(scoreboard.tracked_owners().collect::<Vec<_>>(), ["bob"]);
        // An owner with nothing tracked is not a removal.
        assert_eq!(
            scoreboard.reset_score("alice", "kills"),
            ScoreResetOutcome::Nothing
        );
        assert!(!scoreboard.reset_all_scores("alice"));
        assert!(scoreboard.reset_all_scores("bob"));
        assert_eq!(scoreboard.tracked_owners().count(), 0);
    }

    /// `listPlayerScores(Objective)` (`:166-175`) answers one row per owner
    /// that has a score on it.
    #[test]
    fn the_per_objective_listing_names_only_the_owners_tracking_it() {
        let mut scoreboard = Scoreboard::default();
        scoreboard.add_objective(objective("kills", "dummy"));
        scoreboard.add_objective(objective("deaths", "dummy"));
        scoreboard.score_mut("alice", "kills", false).unwrap().value = 4;
        scoreboard.score_mut("bob", "deaths", false).unwrap().value = 1;
        assert_eq!(
            scoreboard
                .scores_for("kills")
                .into_iter()
                .map(|(owner, score)| (owner.try_to_string().unwrap(), score.value))
                .collect::<Vec<_>>(),
            [("alice".to_string(), 4)]
        );
    }

    /// `setDisplayObjective` (`:237-239`) over an `EnumMap`, so the packed map
    /// comes back in slot order rather than in the order the slots were set.
    #[test]
    fn the_display_slots_are_kept_in_slot_order_and_clear_on_a_null() {
        let mut scoreboard = Scoreboard::default();
        scoreboard.add_objective(objective("kills", "dummy"));
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::TeamRed,
            Some(&bbb_protocol::JavaString::from("kills")),
        );
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::Sidebar,
            Some(&bbb_protocol::JavaString::from("kills")),
        );
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::List,
            Some(&bbb_protocol::JavaString::from("kills")),
        );
        scoreboard.set_display_objective(ScoreboardDisplaySlot::Sidebar, None);
        assert_eq!(
            scoreboard.display_objective(ScoreboardDisplaySlot::Sidebar),
            None
        );
        // Setting the same slot twice replaces rather than appends.
        scoreboard.set_display_objective(
            ScoreboardDisplaySlot::List,
            Some(&bbb_protocol::JavaString::from("kills")),
        );
        assert_eq!(scoreboard.pack().display_slots.len(), 2);
    }

    /// `addPlayerToTeam` (`:272-279`) takes a player off whatever team it was
    /// on first, so membership stays single.
    #[test]
    fn joining_a_team_leaves_the_previous_one() {
        let mut scoreboard = Scoreboard::default();
        assert!(scoreboard.add_team("red"));
        assert!(scoreboard.add_team("blue"));
        // `addPlayerTeam` warns and keeps the existing team (`:250-253`).
        assert!(!scoreboard.add_team("red"));

        assert_eq!(
            scoreboard.add_player_to_team("alice", "red"),
            Some(TeamJoin {
                left: None,
                added: true
            })
        );
        assert_eq!(scoreboard.players_team("alice").unwrap().name, "red");
        // The move reports the team it took the player off, which is where the
        // fan-out's LEAVE packet comes from.
        assert_eq!(
            scoreboard.add_player_to_team("alice", "blue"),
            Some(TeamJoin {
                left: Some("red".into()),
                added: true
            })
        );
        assert_eq!(scoreboard.players_team("alice").unwrap().name, "blue");
        assert!(scoreboard.team_members("red").next().is_none());
        // Rejoining the team the player is already on still leaves it first
        // (`:273-275`), so the set has room to grow again.
        assert_eq!(
            scoreboard.add_player_to_team("alice", "blue"),
            Some(TeamJoin {
                left: Some("blue".into()),
                added: true
            })
        );
        // A team no name carries takes nobody.
        assert_eq!(scoreboard.add_player_to_team("alice", "green"), None);

        assert!(scoreboard.remove_team("blue"));
        assert!(scoreboard.players_team("alice").is_none());
        assert!(!scoreboard.remove_team("blue"));
    }

    #[test]
    fn removing_a_team_keeps_raw_members_and_rules_consistent() {
        let raw = [0xd800, 0xd801, 0xfffd].map(|unit| JavaString::from(vec![unit, 0x61]));
        let mut board = Scoreboard::default();
        board.add_team("red");
        board.add_team("blue");
        board.add_player_to_team("alice", "red");
        board.add_player_to_team("bob", "blue");
        for member in &raw {
            board.add_player_to_team(member, "blue");
        }
        board.team_mut("blue").unwrap().collision_rule = TeamCollisionRule::Never;

        board.remove_team("red");
        assert_eq!(board.teams.team_of("alice"), None);
        assert!(board.teams.pushes_nothing(board.teams.team_of("bob")));
        assert_eq!(board.players_team("bob").unwrap().name, "blue");
        let packed = board.pack();
        assert_eq!(packed.teams[0].players.len(), 4);
        let restored = Scoreboard::restore(&packed);
        for member in &raw {
            assert_eq!(board.players_team(member).unwrap().name, "blue");
            assert!(packed.teams[0].players.contains(member));
            assert_eq!(restored.players_team(member).unwrap().name, "blue");
        }
    }

    fn packed_team(name: &str) -> Java26ScoreboardTeam {
        Java26ScoreboardTeam {
            name: name.into(),
            display_name: None,
            color: None,
            allow_friendly_fire: false,
            see_friendly_invisibles: false,
            member_name_prefix: Some(text("[")),
            member_name_suffix: Some(text("]")),
            name_tag_visibility: TeamVisibility::HideForOtherTeams,
            death_message_visibility: TeamVisibility::Never,
            collision_rule: TeamCollisionRule::PushOwnTeam,
            players: vec!["alice".into(), "bob".into()],
        }
    }

    /// `loadPlayerTeam` (`:379-394`) writes the display name and the colour
    /// only when the packed value carries one, and `pack`
    /// (`PlayerTeam.java:44-58`) always writes the display name back while
    /// omitting a `RESET` colour (`:48`).
    #[test]
    fn a_team_without_a_packed_display_name_keeps_the_constructors() {
        let restored = Scoreboard::restore(&Java26ScoreboardPacked {
            objectives: Vec::new(),
            scores: Vec::new(),
            display_slots: Vec::new(),
            teams: vec![packed_team("red")],
        });
        let team = restored.team("red").expect("restored");
        // `new PlayerTeam` sets the display name to the name itself (`:40`)
        // and the colour to `RESET` (`:33`).
        assert_eq!(team.display_name, text("red"));
        assert_eq!(team.color, ChatFormatting::Reset);
        assert!(!team.allow_friendly_fire);
        assert_eq!(team.death_message_visibility, TeamVisibility::Never);
        let mut members: Vec<_> = restored.team_members("red").collect();
        members.sort();
        assert_eq!(members, ["alice", "bob"]);

        let packed = restored.pack();
        assert_eq!(packed.teams[0].display_name, Some(text("red")));
        assert_eq!(packed.teams[0].color, None);
    }

    /// The whole document survives a restore and a repack.
    #[test]
    fn a_full_document_round_trips() {
        let packed = Java26ScoreboardPacked {
            objectives: vec![
                Java26ScoreboardObjective {
                    name: "kills".into(),
                    criteria: "playerKillCount".to_owned(),
                    display_name: text("Kills"),
                    render_type: ObjectiveRenderType::Integer,
                    display_auto_update: true,
                    format: Some(Java26ScoreboardNumberFormat::Blank),
                },
                Java26ScoreboardObjective {
                    name: "hp".into(),
                    criteria: "health".to_owned(),
                    display_name: text("Health"),
                    render_type: ObjectiveRenderType::Hearts,
                    display_auto_update: false,
                    format: None,
                },
            ],
            scores: vec![
                Java26ScoreboardPackedScore {
                    owner: "alice".into(),
                    objective: "kills".into(),
                    score: 7,
                    locked: true,
                    display: Some(text("Alice")),
                    format: Some(Java26ScoreboardNumberFormat::Fixed(text("7"))),
                },
                // A read-only criterion's score is restored the same way, the
                // load path filing the packed instance rather than writing it.
                Java26ScoreboardPackedScore {
                    owner: "alice".into(),
                    objective: "hp".into(),
                    score: 20,
                    locked: false,
                    display: None,
                    format: None,
                },
            ],
            display_slots: vec![(ScoreboardDisplaySlot::Sidebar, "kills".into())],
            teams: vec![Java26ScoreboardTeam {
                display_name: Some(text("Red")),
                color: Some(ChatFormatting::Red),
                ..packed_team("red")
            }],
        };
        let mut restored = Scoreboard::restore(&packed).pack();
        for team in &mut restored.teams {
            team.players.sort();
        }
        assert_eq!(restored, packed);
    }
}
