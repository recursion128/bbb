use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::{
    chat_formattings::{
        vanilla_chat_formatting_color_by_name, ChatFormatting, VANILLA_CHAT_FORMATTINGS,
    },
    packets::{
        ChatFormatting as ProtocolChatFormatting, NumberFormatSummary,
        ObjectiveRenderType as ProtocolObjectiveRenderType,
        PlayerTeamMethod as ProtocolPlayerTeamMethod,
        PlayerTeamParameters as ProtocolPlayerTeamParameters, ResetScore as ProtocolResetScore,
        ScoreboardDisplaySlot as ProtocolScoreboardDisplaySlot,
        SetDisplayObjective as ProtocolSetDisplayObjective, SetObjective as ProtocolSetObjective,
        SetObjectiveMethod as ProtocolSetObjectiveMethod, SetPlayerTeam as ProtocolSetPlayerTeam,
        SetScore as ProtocolSetScore, TeamCollisionRule as ProtocolTeamCollisionRule,
        TeamVisibility as ProtocolTeamVisibility,
    },
    ComponentStyle, NbtSummaryValue, RichTextComponent, StyledTextRun,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardState {
    pub objectives: BTreeMap<String, ScoreboardObjective>,
    pub display_slots: BTreeMap<String, String>,
    pub scores: BTreeMap<String, BTreeMap<String, ScoreboardScore>>,
    pub teams: BTreeMap<String, ScoreboardTeam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardObjective {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub display_name_styled: Vec<StyledTextRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_component: Option<NbtSummaryValue>,
    pub render_type: String,
    pub number_format: Option<NumberFormatSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardScore {
    pub value: i32,
    pub display: Option<String>,
    #[serde(default)]
    pub display_styled: Option<Vec<StyledTextRun>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_component: Option<NbtSummaryValue>,
    pub number_format: Option<NumberFormatSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardTeam {
    pub name: String,
    pub parameters: Option<ScoreboardTeamParameters>,
    pub players: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreboardTeamParameters {
    pub display_name: String,
    #[serde(default)]
    pub display_name_styled: Vec<StyledTextRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_component: Option<NbtSummaryValue>,
    pub options: i32,
    pub nametag_visibility: String,
    pub collision_rule: String,
    pub color: String,
    pub player_prefix: String,
    #[serde(default)]
    pub player_prefix_styled: Vec<StyledTextRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_prefix_component: Option<NbtSummaryValue>,
    pub player_suffix: String,
    #[serde(default)]
    pub player_suffix_styled: Vec<StyledTextRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_suffix_component: Option<NbtSummaryValue>,
}

/// One canonical scoreboard Component fragment before native localization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreboardTextSegmentState {
    Rich(RichTextComponent),
    Styled(Vec<StyledTextRun>),
}

/// A composed scoreboard line. Team formatting is inherited by each child
/// fragment, so explicit child styles continue to win like vanilla Component
/// style inheritance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScoreboardTextState {
    pub segments: Vec<ScoreboardTextSegmentState>,
    pub inherited_style: ComponentStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreboardSidebarEntryState {
    pub owner: String,
    pub value: i32,
    pub name: ScoreboardTextState,
    pub score: ScoreboardTextState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreboardSidebarState {
    pub objective_name: String,
    pub title: ScoreboardTextState,
    pub entries: Vec<ScoreboardSidebarEntryState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScoreboardPlayerListScoreState {
    Integer(ScoreboardTextState),
    Hearts(i32),
}

impl ScoreboardState {
    pub fn team_for_scoreboard_name(&self, scoreboard_name: &str) -> Option<&ScoreboardTeam> {
        self.teams
            .values()
            .find(|team| team.players.contains(scoreboard_name))
    }

    pub fn team_color_rgb_for_scoreboard_name(&self, scoreboard_name: &str) -> Option<u32> {
        let team = self.team_for_scoreboard_name(scoreboard_name)?;
        let parameters = team.parameters.as_ref()?;
        chat_formatting_rgb(parameters.color.as_str())
    }

    /// Vanilla `Gui.extractScoreboardSidebar`: a color-specific team slot
    /// overrides the generic sidebar when that slot has an objective.
    pub fn sidebar_state(
        &self,
        local_scoreboard_name: Option<&str>,
    ) -> Option<ScoreboardSidebarState> {
        let team_slot = local_scoreboard_name
            .and_then(|name| self.team_for_scoreboard_name(name))
            .and_then(|team| team.parameters.as_ref())
            .filter(|parameters| chat_formatting_rgb(&parameters.color).is_some())
            .map(|parameters| format!("sidebar.team.{}", parameters.color));
        let objective_name = team_slot
            .as_deref()
            .and_then(|slot| self.display_slots.get(slot))
            .or_else(|| self.display_slots.get("sidebar"))?;
        let objective = self.objectives.get(objective_name)?;
        let mut scores = self
            .scores
            .iter()
            .filter(|(owner, _)| !owner.starts_with('#'))
            .filter_map(|(owner, scores)| {
                scores
                    .get(objective_name)
                    .map(|score| (owner.as_str(), score))
            })
            .collect::<Vec<_>>();
        scores.sort_by(|(left_owner, left), (right_owner, right)| {
            right
                .value
                .cmp(&left.value)
                .then_with(|| left_owner.to_lowercase().cmp(&right_owner.to_lowercase()))
        });

        let entries = scores
            .into_iter()
            .take(15)
            .map(|(owner, score)| ScoreboardSidebarEntryState {
                owner: owner.to_string(),
                value: score.value,
                name: self.team_formatted_name(owner, scoreboard_owner_name_segment(owner, score)),
                score: formatted_score_text(
                    score.value,
                    score.number_format.as_ref(),
                    objective.number_format.as_ref(),
                    "red",
                ),
            })
            .collect();
        Some(ScoreboardSidebarState {
            objective_name: objective_name.clone(),
            title: stored_scoreboard_text(
                objective.display_name_component.as_ref(),
                &objective.display_name_styled,
                &objective.display_name,
            ),
            entries,
        })
    }

    /// Vanilla `PlayerTabOverlay` LIST objective score. Missing owner scores
    /// are formatted as zero through `ReadOnlyScoreInfo.safeFormatValue`.
    pub fn player_list_score(&self, owner: &str) -> Option<ScoreboardPlayerListScoreState> {
        let objective_name = self.display_slots.get("list")?;
        let objective = self.objectives.get(objective_name)?;
        let score = self
            .scores
            .get(owner)
            .and_then(|scores| scores.get(objective_name));
        let value = score.map_or(0, |score| score.value);
        if objective.render_type == "hearts" {
            return Some(ScoreboardPlayerListScoreState::Hearts(value));
        }
        Some(ScoreboardPlayerListScoreState::Integer(
            formatted_score_text(
                value,
                score.and_then(|score| score.number_format.as_ref()),
                objective.number_format.as_ref(),
                "yellow",
            ),
        ))
    }

    pub fn team_formatted_literal_name(&self, scoreboard_name: &str) -> ScoreboardTextState {
        self.team_formatted_name(
            scoreboard_name,
            ScoreboardTextSegmentState::Styled(vec![plain_run(scoreboard_name)]),
        )
    }

    pub fn same_team_can_see_friendly_invisibles(
        &self,
        viewer_scoreboard_name: &str,
        target_scoreboard_name: &str,
    ) -> bool {
        let Some(viewer_team) = self.team_for_scoreboard_name(viewer_scoreboard_name) else {
            return false;
        };
        let Some(target_team) = self.team_for_scoreboard_name(target_scoreboard_name) else {
            return false;
        };

        viewer_team.name == target_team.name
            && target_team
                .parameters
                .as_ref()
                .is_some_and(ScoreboardTeamParameters::can_see_friendly_invisibles)
    }

    /// Vanilla `Entity.belowNameDisplay`: format the score override (or the
    /// objective's default, falling back to unstyled decimal zero), append one
    /// space, then preserve the objective display component's inherited runs.
    pub fn below_name_display_runs(&self, scoreboard_name: &str) -> Option<Vec<StyledTextRun>> {
        let objective_name = self.display_slots.get("below_name")?;
        let objective = self.objectives.get(objective_name)?;
        let score = self
            .scores
            .get(scoreboard_name)
            .and_then(|scores| scores.get(objective_name));
        let value = score.map_or(0, |score| score.value);
        let number_format = score
            .and_then(|score| score.number_format.as_ref())
            .or(objective.number_format.as_ref());
        let mut runs = format_score_value(value, number_format);
        runs.push(StyledTextRun {
            text: " ".to_string(),
            style: ComponentStyle::default(),
        });
        if objective.display_name_styled.is_empty() && !objective.display_name.is_empty() {
            runs.push(StyledTextRun {
                text: objective.display_name.clone(),
                style: ComponentStyle::default(),
            });
        } else {
            runs.extend(objective.display_name_styled.iter().cloned());
        }
        Some(runs)
    }

    /// Vanilla `EntitySelector.pushableBy`: applies both teams' collision rules
    /// before a client-owned local-player push. A missing team uses `ALWAYS`.
    pub(crate) fn entity_can_push_scoreboard_target(
        &self,
        pusher_scoreboard_name: &str,
        target_scoreboard_name: Option<&str>,
    ) -> bool {
        let pusher_team = self.team_for_scoreboard_name(pusher_scoreboard_name);
        let pusher_rule = pusher_team
            .and_then(|team| team.parameters.as_ref())
            .map_or("always", |parameters| parameters.collision_rule.as_str());
        if pusher_rule == "never" {
            return false;
        }

        let target_team = target_scoreboard_name
            .and_then(|scoreboard_name| self.team_for_scoreboard_name(scoreboard_name));
        let target_rule = target_team
            .and_then(|team| team.parameters.as_ref())
            .map_or("always", |parameters| parameters.collision_rule.as_str());
        if target_rule == "never" {
            return false;
        }

        let same_team = pusher_team
            .zip(target_team)
            .is_some_and(|(pusher, target)| pusher.name == target.name);
        if same_team && (pusher_rule == "pushOwnTeam" || target_rule == "pushOwnTeam") {
            return false;
        }
        (pusher_rule != "pushOtherTeams" && target_rule != "pushOtherTeams") || same_team
    }

    fn team_formatted_name(
        &self,
        scoreboard_name: &str,
        base: ScoreboardTextSegmentState,
    ) -> ScoreboardTextState {
        let Some(parameters) = self
            .team_for_scoreboard_name(scoreboard_name)
            .and_then(|team| team.parameters.as_ref())
        else {
            return ScoreboardTextState {
                segments: vec![base],
                inherited_style: ComponentStyle::default(),
            };
        };
        ScoreboardTextState {
            segments: vec![
                stored_scoreboard_component(
                    parameters.player_prefix_component.as_ref(),
                    &parameters.player_prefix_styled,
                    &parameters.player_prefix,
                ),
                base,
                stored_scoreboard_component(
                    parameters.player_suffix_component.as_ref(),
                    &parameters.player_suffix_styled,
                    &parameters.player_suffix,
                ),
            ],
            inherited_style: scoreboard_team_formatting_style(&parameters.color),
        }
    }
}

impl ScoreboardTeamParameters {
    pub fn can_see_friendly_invisibles(&self) -> bool {
        self.options & 2 != 0
    }
}

impl WorldStore {
    pub fn apply_set_objective(&mut self, packet: ProtocolSetObjective) -> bool {
        self.counters.set_objective_packets += 1;

        let applied = match packet.method.known() {
            Some(ProtocolSetObjectiveMethod::Add) => {
                let Some(parameters) = packet.parameters else {
                    return self.finish_set_objective_update(false);
                };
                if self
                    .scoreboard
                    .objectives
                    .contains_key(&packet.objective_name)
                {
                    return self.finish_set_objective_update(false);
                }
                let (display_name, display_name_styled, display_name_component) =
                    rich_text_projection(parameters.display_name);
                let objective = ScoreboardObjective {
                    name: packet.objective_name.clone(),
                    display_name,
                    display_name_styled,
                    display_name_component: Some(display_name_component),
                    render_type: objective_render_type_name(parameters.render_type),
                    number_format: parameters.number_format,
                };
                self.scoreboard
                    .objectives
                    .insert(packet.objective_name, objective);
                true
            }
            Some(ProtocolSetObjectiveMethod::Remove) => {
                self.remove_scoreboard_objective(&packet.objective_name)
            }
            Some(ProtocolSetObjectiveMethod::Change) => {
                let Some(parameters) = packet.parameters else {
                    return self.finish_set_objective_update(false);
                };
                let Some(objective) = self.scoreboard.objectives.get_mut(&packet.objective_name)
                else {
                    return self.finish_set_objective_update(false);
                };
                let (display_name, display_name_styled, display_name_component) =
                    rich_text_projection(parameters.display_name);
                objective.display_name = display_name;
                objective.display_name_styled = display_name_styled;
                objective.display_name_component = Some(display_name_component);
                objective.render_type = objective_render_type_name(parameters.render_type);
                objective.number_format = parameters.number_format;
                true
            }
            None => false,
        };
        self.finish_set_objective_update(applied)
    }

    pub fn apply_set_score(&mut self, packet: ProtocolSetScore) -> bool {
        self.counters.set_score_packets += 1;
        if !self
            .scoreboard
            .objectives
            .contains_key(&packet.objective_name)
        {
            return self.finish_set_score_update(false);
        }

        let (display, display_styled, display_component) =
            packet.display.map_or((None, None, None), |component| {
                let (text, styled, exact) = rich_text_projection(component);
                (Some(text), Some(styled), Some(exact))
            });
        self.scoreboard
            .scores
            .entry(packet.owner)
            .or_default()
            .insert(
                packet.objective_name,
                ScoreboardScore {
                    value: packet.score,
                    display,
                    display_styled,
                    display_component,
                    number_format: packet.number_format,
                },
            );
        self.finish_set_score_update(true)
    }

    pub fn apply_reset_score(&mut self, packet: ProtocolResetScore) -> bool {
        self.counters.reset_score_packets += 1;
        let Some(objective_name) = packet.objective_name else {
            let applied = self.scoreboard.scores.remove(&packet.owner).is_some();
            return self.finish_reset_score_update(applied);
        };
        if !self.scoreboard.objectives.contains_key(&objective_name) {
            return self.finish_reset_score_update(false);
        }

        let Some(scores) = self.scoreboard.scores.get_mut(&packet.owner) else {
            return self.finish_reset_score_update(false);
        };
        let removed = scores.remove(&objective_name).is_some();
        if scores.is_empty() {
            self.scoreboard.scores.remove(&packet.owner);
        }
        self.finish_reset_score_update(removed)
    }

    pub fn apply_set_display_objective(&mut self, packet: ProtocolSetDisplayObjective) -> bool {
        self.counters.set_display_objective_packets += 1;
        let slot = scoreboard_display_slot_name(packet.slot);
        let Some(objective_name) = packet.objective_name.filter(|name| !name.is_empty()) else {
            let applied = self.scoreboard.display_slots.remove(&slot).is_some();
            return self.finish_set_display_objective_update(applied);
        };
        if !self.scoreboard.objectives.contains_key(&objective_name) {
            let applied = self.scoreboard.display_slots.remove(&slot).is_some();
            return self.finish_set_display_objective_update(applied);
        }

        self.scoreboard.display_slots.insert(slot, objective_name);
        self.finish_set_display_objective_update(true)
    }

    pub fn apply_set_player_team(&mut self, packet: ProtocolSetPlayerTeam) -> bool {
        self.counters.set_player_team_packets += 1;

        let applied = match packet.method.known() {
            Some(ProtocolPlayerTeamMethod::Add) => {
                self.scoreboard
                    .teams
                    .entry(packet.name.clone())
                    .or_insert_with(|| ScoreboardTeam {
                        name: packet.name.clone(),
                        parameters: None,
                        players: BTreeSet::new(),
                    });
                if let Some(parameters) = packet.parameters {
                    if let Some(team) = self.scoreboard.teams.get_mut(&packet.name) {
                        team.parameters = Some(scoreboard_team_parameters(parameters));
                    }
                }
                self.add_players_to_scoreboard_team(&packet.name, packet.players);
                true
            }
            Some(ProtocolPlayerTeamMethod::Remove) => {
                self.scoreboard.teams.remove(&packet.name).is_some()
            }
            Some(ProtocolPlayerTeamMethod::Change) => {
                let Some(team) = self.scoreboard.teams.get_mut(&packet.name) else {
                    return self.finish_set_player_team_update(false);
                };
                let Some(parameters) = packet.parameters else {
                    return self.finish_set_player_team_update(false);
                };
                team.parameters = Some(scoreboard_team_parameters(parameters));
                true
            }
            Some(ProtocolPlayerTeamMethod::Join) => {
                if !self.scoreboard.teams.contains_key(&packet.name) {
                    return self.finish_set_player_team_update(false);
                }
                self.add_players_to_scoreboard_team(&packet.name, packet.players);
                true
            }
            Some(ProtocolPlayerTeamMethod::Leave) => {
                let Some(team) = self.scoreboard.teams.get_mut(&packet.name) else {
                    return self.finish_set_player_team_update(false);
                };
                for player in packet.players {
                    team.players.remove(&player);
                }
                true
            }
            None => false,
        };
        self.finish_set_player_team_update(applied)
    }

    pub fn scoreboard(&self) -> &ScoreboardState {
        &self.scoreboard
    }

    pub fn scoreboard_sidebar(&self) -> Option<ScoreboardSidebarState> {
        let local_scoreboard_name = self.local_player_scoreboard_name();
        self.scoreboard
            .sidebar_state(local_scoreboard_name.as_deref())
    }

    fn remove_scoreboard_objective(&mut self, objective_name: &str) -> bool {
        if self.scoreboard.objectives.remove(objective_name).is_none() {
            return false;
        }

        self.scoreboard
            .display_slots
            .retain(|_, displayed_objective| displayed_objective != objective_name);
        self.scoreboard.scores.retain(|_, scores| {
            scores.remove(objective_name);
            !scores.is_empty()
        });
        true
    }

    fn add_players_to_scoreboard_team(&mut self, team_name: &str, players: Vec<String>) {
        for player in players {
            self.remove_scoreboard_player_from_other_teams(team_name, &player);
            if let Some(team) = self.scoreboard.teams.get_mut(team_name) {
                team.players.insert(player);
            }
        }
    }

    fn remove_scoreboard_player_from_other_teams(&mut self, team_name: &str, player: &str) {
        for (name, team) in &mut self.scoreboard.teams {
            if name.as_str() != team_name {
                team.players.remove(player);
            }
        }
    }

    fn finish_set_objective_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_objective_updates_applied += 1;
        } else {
            self.counters.set_objective_updates_ignored += 1;
        }
        applied
    }

    fn finish_set_score_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_score_updates_applied += 1;
        } else {
            self.counters.set_score_updates_ignored += 1;
        }
        applied
    }

    fn finish_reset_score_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.reset_score_updates_applied += 1;
        } else {
            self.counters.reset_score_updates_ignored += 1;
        }
        applied
    }

    fn finish_set_display_objective_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_display_objective_updates_applied += 1;
        } else {
            self.counters.set_display_objective_updates_ignored += 1;
        }
        applied
    }

    fn finish_set_player_team_update(&mut self, applied: bool) -> bool {
        if applied {
            self.counters.set_player_team_updates_applied += 1;
        } else {
            self.counters.set_player_team_updates_ignored += 1;
        }
        applied
    }
}

fn objective_render_type_name(render_type: ProtocolObjectiveRenderType) -> String {
    render_type.as_str().to_string()
}

fn scoreboard_display_slot_name(slot: ProtocolScoreboardDisplaySlot) -> String {
    slot.as_str().to_string()
}

fn team_visibility_name(visibility: ProtocolTeamVisibility) -> String {
    visibility.as_str().to_string()
}

fn team_collision_rule_name(rule: ProtocolTeamCollisionRule) -> String {
    rule.as_str().to_string()
}

fn chat_formatting_name(color: ProtocolChatFormatting) -> String {
    color.as_str().to_string()
}

fn chat_formatting_rgb(color: &str) -> Option<u32> {
    vanilla_chat_formatting_color_by_name(color)
}

pub(crate) fn scoreboard_team_formatting_style(name: &str) -> ComponentStyle {
    let Some(formatting) = VANILLA_CHAT_FORMATTINGS
        .iter()
        .find(|entry| entry.serialized_name == name)
        .map(|entry| entry.formatting)
    else {
        return ComponentStyle::default();
    };
    let mut style = ComponentStyle::default();
    if let Some(color) = formatting.color() {
        style.color = Some(color);
        return style;
    }
    match formatting {
        ChatFormatting::Obfuscated => style.obfuscated = Some(true),
        ChatFormatting::Bold => style.bold = Some(true),
        ChatFormatting::Strikethrough => style.strikethrough = Some(true),
        ChatFormatting::Underline => style.underlined = Some(true),
        ChatFormatting::Italic => style.italic = Some(true),
        _ => {}
    }
    style
}

fn scoreboard_team_parameters(
    parameters: ProtocolPlayerTeamParameters,
) -> ScoreboardTeamParameters {
    let (display_name, display_name_styled, display_name_component) =
        rich_text_projection(parameters.display_name);
    let (player_prefix, player_prefix_styled, player_prefix_component) =
        rich_text_projection(parameters.player_prefix);
    let (player_suffix, player_suffix_styled, player_suffix_component) =
        rich_text_projection(parameters.player_suffix);
    ScoreboardTeamParameters {
        display_name,
        display_name_styled,
        display_name_component: Some(display_name_component),
        options: i32::from(parameters.options),
        nametag_visibility: team_visibility_name(parameters.nametag_visibility),
        collision_rule: team_collision_rule_name(parameters.collision_rule),
        color: chat_formatting_name(parameters.color),
        player_prefix,
        player_prefix_styled,
        player_prefix_component: Some(player_prefix_component),
        player_suffix,
        player_suffix_styled,
        player_suffix_component: Some(player_suffix_component),
    }
}

fn rich_text_projection(
    component: RichTextComponent,
) -> (String, Vec<StyledTextRun>, NbtSummaryValue) {
    let text = if matches!(component.component(), NbtSummaryValue::String(value) if value.is_empty())
    {
        String::new()
    } else {
        component.text().to_string()
    };
    (
        text,
        component.styled().to_vec(),
        component.component().clone(),
    )
}

fn scoreboard_owner_name_segment(
    owner: &str,
    score: &ScoreboardScore,
) -> ScoreboardTextSegmentState {
    score.display_component.as_ref().map_or_else(
        || ScoreboardTextSegmentState::Styled(vec![plain_run(owner)]),
        |component| {
            stored_scoreboard_component(
                Some(component),
                score.display_styled.as_deref().unwrap_or_default(),
                score.display.as_deref().unwrap_or(owner),
            )
        },
    )
}

fn formatted_score_text(
    value: i32,
    score_format: Option<&NumberFormatSummary>,
    objective_format: Option<&NumberFormatSummary>,
    default_color: &str,
) -> ScoreboardTextState {
    let format = score_format.or(objective_format);
    let segments = match format {
        Some(NumberFormatSummary::Blank) => Vec::new(),
        Some(NumberFormatSummary::Styled(style)) => {
            vec![ScoreboardTextSegmentState::Styled(vec![StyledTextRun {
                text: value.to_string(),
                style: style.clone(),
            }])]
        }
        Some(NumberFormatSummary::Fixed(component)) => {
            vec![ScoreboardTextSegmentState::Rich(component.clone())]
        }
        None => vec![ScoreboardTextSegmentState::Styled(vec![StyledTextRun {
            text: value.to_string(),
            style: ComponentStyle {
                color: chat_formatting_rgb(default_color),
                ..ComponentStyle::default()
            },
        }])],
    };
    ScoreboardTextState {
        segments,
        inherited_style: ComponentStyle::default(),
    }
}

fn stored_scoreboard_text(
    exact: Option<&NbtSummaryValue>,
    styled: &[StyledTextRun],
    plain: &str,
) -> ScoreboardTextState {
    ScoreboardTextState {
        segments: vec![stored_scoreboard_component(exact, styled, plain)],
        inherited_style: ComponentStyle::default(),
    }
}

fn stored_scoreboard_component(
    exact: Option<&NbtSummaryValue>,
    styled: &[StyledTextRun],
    plain: &str,
) -> ScoreboardTextSegmentState {
    if let Some(component) = exact
        .cloned()
        .and_then(|component| RichTextComponent::try_from_component_nbt(component).ok())
    {
        return ScoreboardTextSegmentState::Rich(component);
    }
    if !styled.is_empty() {
        ScoreboardTextSegmentState::Styled(styled.to_vec())
    } else {
        ScoreboardTextSegmentState::Styled(vec![plain_run(plain)])
    }
}

fn plain_run(text: impl Into<String>) -> StyledTextRun {
    StyledTextRun {
        text: text.into(),
        style: ComponentStyle::default(),
    }
}

fn format_score_value(
    value: i32,
    number_format: Option<&NumberFormatSummary>,
) -> Vec<StyledTextRun> {
    match number_format {
        Some(NumberFormatSummary::Blank) => Vec::new(),
        Some(NumberFormatSummary::Styled(style)) => vec![StyledTextRun {
            text: value.to_string(),
            style: style.clone(),
        }],
        Some(NumberFormatSummary::Fixed(component)) => component.styled().to_vec(),
        None => vec![StyledTextRun {
            text: value.to_string(),
            style: ComponentStyle::default(),
        }],
    }
}

#[cfg(test)]
mod tests;
