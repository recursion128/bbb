use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_net::NetCommand;
use bbb_protocol::packets::TeleportToEntity;
use bbb_world::{PlayerInfoProfileState, ScoreboardTextState, WorldStore};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::{commands::queue_teleport_to_entity_command, ClientInputState};

const SPECTATOR_MENU_SLOT_COUNT: usize = 9;
const SPECTATOR_MENU_ITEMS_PER_PAGE: usize = 6;
const SPECTATOR_MENU_PREVIOUS_SLOT: u8 = 0;
const SPECTATOR_MENU_NEXT_SLOT: u8 = 7;
const SPECTATOR_MENU_CLOSE_SLOT: u8 = 8;
const SPECTATOR_MENU_FADE_OUT_DELAY: Duration = Duration::from_millis(5_000);

#[derive(Debug, Clone, Default)]
pub(crate) struct SpectatorGuiInputState {
    menu: Option<SpectatorMenuInputState>,
    last_selection_at: Option<Instant>,
    icon_random_state: u64,
}

#[derive(Debug, Clone)]
struct SpectatorMenuInputState {
    catalog: SpectatorMenuCatalog,
    category: SpectatorMenuCategory,
    selected_slot: i8,
    page: usize,
}

#[derive(Debug, Clone, Default)]
struct SpectatorMenuCatalog {
    players: Vec<SpectatorMenuPlayer>,
    teams: Vec<SpectatorMenuTeam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpectatorMenuPlayer {
    pub(crate) profile: PlayerInfoProfileState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpectatorMenuTeam {
    pub(crate) display_name: ScoreboardTextState,
    pub(crate) color_rgb: Option<u32>,
    pub(crate) icon_player: SpectatorMenuPlayer,
    players: Vec<SpectatorMenuPlayer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpectatorMenuCategory {
    Root,
    Players,
    TeamList,
    Team(usize),
}

#[derive(Debug, Clone, Copy)]
enum SpectatorMenuItem {
    Empty,
    PlayersCategory { enabled: bool },
    TeamsCategory { enabled: bool },
    Player(Uuid),
    Team(usize),
    PreviousPage,
    NextPage { enabled: bool },
    Close,
}

impl SpectatorMenuItem {
    fn present(self) -> bool {
        !matches!(self, Self::Empty)
    }

    fn enabled(self) -> bool {
        match self {
            Self::Empty => false,
            Self::PlayersCategory { enabled }
            | Self::TeamsCategory { enabled }
            | Self::NextPage { enabled } => enabled,
            Self::Player(_) | Self::Team(_) | Self::PreviousPage | Self::Close => true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SpectatorMenuOutcome {
    teleport_to: Option<Uuid>,
    close: bool,
}

impl SpectatorMenuCatalog {
    fn from_world(world: &WorldStore, icon_random_state: &mut u64) -> Self {
        let mut players = world
            .listed_players()
            .iter()
            .filter_map(|profile_id| world.player_info_entry(*profile_id))
            .filter(|entry| !entry.is_spectator())
            .map(|entry| SpectatorMenuPlayer {
                profile: entry.profile.clone(),
            })
            .collect::<Vec<_>>();
        players.sort_by_key(|player| player.profile.uuid);

        let mut teams = Vec::new();
        for (team_name, team) in &world.scoreboard().teams {
            let mut team_players = team
                .players
                .iter()
                .filter_map(|name| {
                    world
                        .player_info()
                        .entries
                        .values()
                        .find(|entry| entry.profile.name == *name)
                })
                .filter(|entry| !entry.is_spectator())
                .map(|entry| SpectatorMenuPlayer {
                    profile: entry.profile.clone(),
                })
                .collect::<Vec<_>>();
            if team_players.is_empty() {
                continue;
            }
            team_players.sort_by_key(|player| player.profile.uuid);
            let icon_index = next_icon_index(icon_random_state, team_players.len());
            teams.push(SpectatorMenuTeam {
                display_name: world
                    .scoreboard()
                    .team_display_name(team_name)
                    .expect("iterated scoreboard team has a display name"),
                color_rgb: world.scoreboard().team_color_rgb(team_name),
                icon_player: team_players[icon_index].clone(),
                players: team_players,
            });
        }

        Self { players, teams }
    }
}

fn next_icon_index(state: &mut u64, len: usize) -> usize {
    debug_assert!(len > 0);
    // Runtime-only deterministic LCG. Vanilla chooses one eligible member
    // through its thread-local RandomSource each time the root is constructed;
    // the observable contract is a member of this captured team.
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    (*state as usize) % len
}

impl SpectatorMenuInputState {
    fn from_world(world: &WorldStore, icon_random_state: &mut u64) -> Self {
        Self {
            catalog: SpectatorMenuCatalog::from_world(world, icon_random_state),
            category: SpectatorMenuCategory::Root,
            selected_slot: -1,
            page: 0,
        }
    }

    fn category_len(&self) -> usize {
        match self.category {
            SpectatorMenuCategory::Root => 2,
            SpectatorMenuCategory::Players => self.catalog.players.len(),
            SpectatorMenuCategory::TeamList => self.catalog.teams.len(),
            SpectatorMenuCategory::Team(index) => self
                .catalog
                .teams
                .get(index)
                .map_or(0, |team| team.players.len()),
        }
    }

    fn item(&self, slot: u8) -> SpectatorMenuItem {
        let slot_index = usize::from(slot);
        let index =
            slot_index.saturating_add(self.page.saturating_mul(SPECTATOR_MENU_ITEMS_PER_PAGE));
        if self.page > 0 && slot == SPECTATOR_MENU_PREVIOUS_SLOT {
            return SpectatorMenuItem::PreviousPage;
        }
        if slot == SPECTATOR_MENU_NEXT_SLOT {
            return SpectatorMenuItem::NextPage {
                enabled: index < self.category_len(),
            };
        }
        if slot == SPECTATOR_MENU_CLOSE_SLOT {
            return SpectatorMenuItem::Close;
        }
        match self.category {
            SpectatorMenuCategory::Root => match index {
                0 => SpectatorMenuItem::PlayersCategory {
                    enabled: !self.catalog.players.is_empty(),
                },
                1 => SpectatorMenuItem::TeamsCategory {
                    enabled: !self.catalog.teams.is_empty(),
                },
                _ => SpectatorMenuItem::Empty,
            },
            SpectatorMenuCategory::Players => self
                .catalog
                .players
                .get(index)
                .map_or(SpectatorMenuItem::Empty, |player| {
                    SpectatorMenuItem::Player(player.profile.uuid)
                }),
            SpectatorMenuCategory::TeamList => self
                .catalog
                .teams
                .get(index)
                .map_or(SpectatorMenuItem::Empty, |_| SpectatorMenuItem::Team(index)),
            SpectatorMenuCategory::Team(team_index) => self
                .catalog
                .teams
                .get(team_index)
                .and_then(|team| team.players.get(index))
                .map_or(SpectatorMenuItem::Empty, |player| {
                    SpectatorMenuItem::Player(player.profile.uuid)
                }),
        }
    }

    fn select_slot(&mut self, slot: u8) -> SpectatorMenuOutcome {
        let item = self.item(slot);
        if !item.present() {
            return SpectatorMenuOutcome::default();
        }
        if self.selected_slot == slot as i8 && item.enabled() {
            return self.activate(item);
        }
        self.selected_slot = slot as i8;
        SpectatorMenuOutcome::default()
    }

    fn activate(&mut self, item: SpectatorMenuItem) -> SpectatorMenuOutcome {
        match item {
            SpectatorMenuItem::PlayersCategory { .. } => {
                self.select_category(SpectatorMenuCategory::Players);
                SpectatorMenuOutcome::default()
            }
            SpectatorMenuItem::TeamsCategory { .. } => {
                // The team category's items are the captured root team list.
                self.select_category(SpectatorMenuCategory::TeamList);
                SpectatorMenuOutcome::default()
            }
            SpectatorMenuItem::Player(uuid) => SpectatorMenuOutcome {
                teleport_to: Some(uuid),
                close: false,
            },
            SpectatorMenuItem::Team(index) => {
                self.select_category(SpectatorMenuCategory::Team(index));
                SpectatorMenuOutcome::default()
            }
            SpectatorMenuItem::PreviousPage => {
                self.page = self.page.saturating_sub(1);
                SpectatorMenuOutcome::default()
            }
            SpectatorMenuItem::NextPage { .. } => {
                self.page = self.page.saturating_add(1);
                SpectatorMenuOutcome::default()
            }
            SpectatorMenuItem::Close => SpectatorMenuOutcome {
                teleport_to: None,
                close: true,
            },
            SpectatorMenuItem::Empty => SpectatorMenuOutcome::default(),
        }
    }

    fn select_category(&mut self, category: SpectatorMenuCategory) {
        self.category = category;
        self.selected_slot = -1;
        self.page = 0;
    }
}

impl ClientInputState {
    pub(crate) fn handle_spectator_hotbar_selected(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        slot: u8,
    ) -> bool {
        self.handle_spectator_hotbar_selected_at(
            world,
            counters,
            net_commands,
            slot,
            Instant::now(),
        )
    }

    fn handle_spectator_hotbar_selected_at(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        slot: u8,
        now: Instant,
    ) -> bool {
        if !world.local_player_is_spectator() || usize::from(slot) >= SPECTATOR_MENU_SLOT_COUNT {
            return false;
        }
        self.expire_spectator_menu_at(now);
        self.spectator_gui.last_selection_at = Some(now);
        let outcome = if let Some(menu) = self.spectator_gui.menu.as_mut() {
            menu.select_slot(slot)
        } else {
            self.spectator_gui.menu = Some(SpectatorMenuInputState::from_world(
                world,
                &mut self.spectator_gui.icon_random_state,
            ));
            SpectatorMenuOutcome::default()
        };
        self.apply_spectator_menu_outcome(counters, net_commands, outcome);
        true
    }

    pub(crate) fn handle_spectator_hotbar_action(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) -> bool {
        self.handle_spectator_hotbar_action_at(world, counters, net_commands, Instant::now())
    }

    fn handle_spectator_hotbar_action_at(
        &mut self,
        world: &WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        now: Instant,
    ) -> bool {
        if !world.local_player_is_spectator() {
            return false;
        }
        self.expire_spectator_menu_at(now);
        self.spectator_gui.last_selection_at = Some(now);
        let outcome = if let Some(menu) = self.spectator_gui.menu.as_mut() {
            u8::try_from(menu.selected_slot)
                .ok()
                .map_or_else(SpectatorMenuOutcome::default, |slot| menu.select_slot(slot))
        } else {
            self.spectator_gui.menu = Some(SpectatorMenuInputState::from_world(
                world,
                &mut self.spectator_gui.icon_random_state,
            ));
            SpectatorMenuOutcome::default()
        };
        self.apply_spectator_menu_outcome(counters, net_commands, outcome);
        true
    }

    pub(crate) fn handle_spectator_menu_scroll(&mut self, world: &WorldStore, wheel: i32) -> bool {
        self.handle_spectator_menu_scroll_at(world, wheel, Instant::now())
    }

    fn handle_spectator_menu_scroll_at(
        &mut self,
        world: &WorldStore,
        wheel: i32,
        now: Instant,
    ) -> bool {
        if !world.local_player_is_spectator() {
            return false;
        }
        self.expire_spectator_menu_at(now);
        let Some(menu) = self.spectator_gui.menu.as_mut() else {
            return false;
        };
        if wheel == 0 {
            return true;
        }
        let mut new_slot = i32::from(menu.selected_slot).saturating_add(wheel);
        while (0..=8).contains(&new_slot) {
            let item = menu.item(new_slot as u8);
            if item.present() && item.enabled() {
                break;
            }
            new_slot = new_slot.saturating_add(wheel);
        }
        if (0..=8).contains(&new_slot) {
            menu.select_slot(new_slot as u8);
            self.spectator_gui.last_selection_at = Some(now);
        }
        true
    }

    pub(crate) fn expire_spectator_menu_at(&mut self, now: Instant) {
        let expired = self
            .spectator_gui
            .last_selection_at
            .and_then(|last| now.checked_duration_since(last))
            .is_some_and(|elapsed| elapsed >= SPECTATOR_MENU_FADE_OUT_DELAY);
        if expired {
            self.close_spectator_menu();
        }
    }

    fn apply_spectator_menu_outcome(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        outcome: SpectatorMenuOutcome,
    ) {
        if let Some(uuid) = outcome.teleport_to {
            queue_teleport_to_entity_command(counters, net_commands, TeleportToEntity { uuid });
        }
        if outcome.close {
            self.close_spectator_menu();
        }
    }

    fn close_spectator_menu(&mut self) {
        self.spectator_gui.menu = None;
        self.spectator_gui.last_selection_at = None;
    }

    #[cfg(test)]
    pub(crate) fn spectator_menu_is_active(&self) -> bool {
        self.spectator_gui.menu.is_some()
    }

    #[cfg(test)]
    pub(crate) fn spectator_menu_selected_slot(&self) -> Option<i8> {
        self.spectator_gui
            .menu
            .as_ref()
            .map(|menu| menu.selected_slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        packets::{
            ChatFormatting, GameEvent, GameProfile, GameType, PlayerInfoAction, PlayerInfoEntry,
            PlayerInfoRemove, PlayerInfoUpdate, PlayerTeamMethod, PlayerTeamParameters,
            SetPlayerTeam, TeamCollisionRule, TeamVisibility,
        },
        RichTextComponent,
    };

    fn set_local_spectator(world: &mut WorldStore) {
        world.apply_game_event(GameEvent {
            event_id: 3,
            param: 3.0,
        });
        assert!(world.local_player_is_spectator());
    }

    fn add_player(
        world: &mut WorldStore,
        uuid: Uuid,
        name: &str,
        listed: bool,
        game_mode: GameType,
    ) {
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::AddPlayer,
                PlayerInfoAction::UpdateListed,
                PlayerInfoAction::UpdateGameMode,
            ],
            entries: vec![PlayerInfoEntry {
                profile_id: uuid,
                profile: Some(GameProfile {
                    uuid,
                    name: name.to_string(),
                    properties: Vec::new(),
                }),
                listed,
                latency: 0,
                game_mode,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        });
    }

    fn add_team(world: &mut WorldStore, name: &str, players: &[&str]) {
        assert!(world.apply_set_player_team(SetPlayerTeam {
            name: name.to_string(),
            method: PlayerTeamMethod::Add.into(),
            parameters: Some(PlayerTeamParameters {
                display_name: RichTextComponent::plain("Builders"),
                options: 0,
                nametag_visibility: TeamVisibility::Always,
                collision_rule: TeamCollisionRule::Always,
                color: ChatFormatting::Aqua,
                player_prefix: RichTextComponent::plain(""),
                player_suffix: RichTextComponent::plain(""),
            }),
            players: players.iter().map(|player| (*player).to_string()).collect(),
        }));
    }

    #[test]
    fn root_snapshots_eligible_players_and_teams_in_vanilla_ordering() {
        let mut world = WorldStore::new();
        let uuid_a = Uuid::from_u128(1);
        let uuid_b = Uuid::from_u128(3);
        let uuid_hidden = Uuid::from_u128(2);
        let uuid_spectator = Uuid::from_u128(4);
        add_player(&mut world, uuid_b, "Bea", true, GameType::Survival);
        add_player(&mut world, uuid_a, "Ada", true, GameType::Creative);
        add_player(
            &mut world,
            uuid_hidden,
            "Hidden",
            false,
            GameType::Adventure,
        );
        add_player(
            &mut world,
            uuid_spectator,
            "Watcher",
            true,
            GameType::Spectator,
        );
        add_team(&mut world, "builders", &["Watcher", "Hidden", "Bea"]);

        let mut random = 0;
        let catalog = SpectatorMenuCatalog::from_world(&world, &mut random);

        assert_eq!(
            catalog
                .players
                .iter()
                .map(|player| player.profile.uuid)
                .collect::<Vec<_>>(),
            vec![uuid_a, uuid_b]
        );
        assert_eq!(catalog.teams.len(), 1);
        assert_eq!(
            catalog.teams[0]
                .players
                .iter()
                .map(|player| player.profile.uuid)
                .collect::<Vec<_>>(),
            vec![uuid_hidden, uuid_b]
        );
        assert!(matches!(
            catalog.teams[0].icon_player.profile.uuid,
            id if id == uuid_hidden || id == uuid_b
        ));
        assert_eq!(catalog.teams[0].color_rgb, Some(0x55_FF_FF));
    }

    #[test]
    fn repeated_hotbar_selection_opens_selects_activates_and_queues_uuid() {
        let mut world = WorldStore::new();
        set_local_spectator(&mut world);
        let profile_id = Uuid::from_u128(0x1234);
        add_player(&mut world, profile_id, "Target", true, GameType::Survival);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);
        let start = Instant::now();

        assert!(input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            0,
            start,
        ));
        let menu = input.spectator_gui.menu.as_ref().expect("opened root");
        assert_eq!(menu.category, SpectatorMenuCategory::Root);
        assert_eq!(menu.selected_slot, -1);

        input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            0,
            start + Duration::from_millis(1),
        );
        assert_eq!(input.spectator_gui.menu.as_ref().unwrap().selected_slot, 0);
        input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            0,
            start + Duration::from_millis(2),
        );
        let menu = input.spectator_gui.menu.as_ref().unwrap();
        assert_eq!(menu.category, SpectatorMenuCategory::Players);
        assert_eq!(menu.selected_slot, -1);
        assert_eq!(
            world.apply_player_info_remove(PlayerInfoRemove {
                profile_ids: vec![profile_id],
            }),
            1
        );

        input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            0,
            start + Duration::from_millis(3),
        );
        input.handle_spectator_hotbar_action_at(
            &world,
            &mut counters,
            &commands,
            start + Duration::from_millis(4),
        );

        assert_eq!(counters.teleport_to_entity_commands_queued, 1);
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::TeleportToEntity(TeleportToEntity { uuid: profile_id })
        );
        assert!(input.spectator_menu_is_active());
    }

    #[test]
    fn team_category_uses_captured_unlisted_members() {
        let mut world = WorldStore::new();
        set_local_spectator(&mut world);
        let profile_id = Uuid::from_u128(7);
        add_player(
            &mut world,
            profile_id,
            "Unlisted",
            false,
            GameType::Survival,
        );
        add_team(&mut world, "builders", &["Unlisted"]);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let commands = None;
        let start = Instant::now();

        input.handle_spectator_hotbar_selected_at(&world, &mut counters, &commands, 1, start);
        input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            1,
            start + Duration::from_millis(1),
        );
        input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            1,
            start + Duration::from_millis(2),
        );
        assert_eq!(
            input.spectator_gui.menu.as_ref().unwrap().category,
            SpectatorMenuCategory::TeamList
        );
        input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            0,
            start + Duration::from_millis(3),
        );
        input.handle_spectator_hotbar_action_at(
            &world,
            &mut counters,
            &commands,
            start + Duration::from_millis(4),
        );
        assert_eq!(
            input.spectator_gui.menu.as_ref().unwrap().category,
            SpectatorMenuCategory::Team(0)
        );
    }

    #[test]
    fn paging_wheel_skip_close_and_five_second_expiry_match_vanilla() {
        let players = (0..8)
            .map(|index| SpectatorMenuPlayer {
                profile: PlayerInfoProfileState {
                    uuid: Uuid::from_u128(index + 1),
                    name: format!("Player{index}"),
                    properties: Vec::new(),
                },
            })
            .collect();
        let mut menu = SpectatorMenuInputState {
            catalog: SpectatorMenuCatalog {
                players,
                teams: Vec::new(),
            },
            category: SpectatorMenuCategory::Players,
            selected_slot: -1,
            page: 0,
        };
        assert!(matches!(
            menu.item(SPECTATOR_MENU_NEXT_SLOT),
            SpectatorMenuItem::NextPage { enabled: true }
        ));
        menu.select_slot(SPECTATOR_MENU_NEXT_SLOT);
        menu.select_slot(SPECTATOR_MENU_NEXT_SLOT);
        assert_eq!(menu.page, 1);
        assert!(matches!(menu.item(0), SpectatorMenuItem::PreviousPage));
        assert!(matches!(
            menu.item(1),
            SpectatorMenuItem::Player(uuid) if uuid == Uuid::from_u128(8)
        ));
        assert!(matches!(
            menu.item(SPECTATOR_MENU_NEXT_SLOT),
            SpectatorMenuItem::NextPage { enabled: false }
        ));

        let mut world = WorldStore::new();
        set_local_spectator(&mut world);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let start = Instant::now();
        input.handle_spectator_hotbar_action_at(&world, &mut counters, &None, start);
        assert!(input.handle_spectator_menu_scroll_at(&world, 1, start + Duration::from_millis(1),));
        assert_eq!(
            input.spectator_gui.menu.as_ref().unwrap().selected_slot,
            SPECTATOR_MENU_CLOSE_SLOT as i8
        );
        input.expire_spectator_menu_at(start + Duration::from_millis(5_000));
        assert!(input.spectator_menu_is_active());
        input.expire_spectator_menu_at(start + Duration::from_millis(5_001));
        assert!(!input.spectator_menu_is_active());
    }

    #[test]
    fn non_spectator_never_opens_menu_or_sends_teleport() {
        let world = WorldStore::new();
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let (tx, mut rx) = mpsc::channel(1);
        let commands = Some(tx);

        assert!(!input.handle_spectator_hotbar_selected_at(
            &world,
            &mut counters,
            &commands,
            0,
            Instant::now(),
        ));
        assert!(!input.spectator_menu_is_active());
        assert_eq!(counters.teleport_to_entity_commands_queued, 0);
        assert!(rx.try_recv().is_err());
    }
}
