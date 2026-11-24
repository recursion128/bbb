use bbb_client_clock::Instant;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use bbb_client_render::{
    HudSocialInteractionsFocus, HudSocialInteractionsHitTarget, HudSocialInteractionsHitTargetKind,
    HudSocialInteractionsPage, HudSocialInteractionsPlayerAction,
};
use bbb_client_world::{SocialInteractionsPage, SocialPlayerState, WorldStore};
use uuid::Uuid;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use bbb_client_net::NetCommand;
use tokio::sync::mpsc;

use super::{
    release_active_input, text_field::TextField, ClientInputState, DebugClipboard, Screen,
};
use crate::control::NetCounters;

pub(crate) const BLOCKING_HELP_URL: &str = "https://aka.ms/javablocking";
const MAX_SEARCH_LENGTH: usize = 16;
const ROW_HEIGHT: i32 = 36;
const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SocialInteractionsPlatformRequest {
    OpenUri(String),
    ClipboardWrite(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SocialSyncRevision {
    player_list_display_revision: u64,
    chat_delivery_revision: u64,
    chat_display_refresh_revision: u64,
    local_player: Option<Uuid>,
    hidden_players: Vec<Uuid>,
    blocked_players: Vec<Uuid>,
}

impl SocialSyncRevision {
    fn from_world(world: &WorldStore) -> Self {
        let chat = world.client_chat();
        Self {
            player_list_display_revision: world.player_list_display_revision(),
            chat_delivery_revision: chat.delivery_revision,
            chat_display_refresh_revision: chat.display_refresh_revision,
            local_player: world.local_player_profile_id(),
            hidden_players: chat.presentation.hidden_players.clone(),
            blocked_players: chat.presentation.blocked_players.clone(),
        }
    }

    fn matches(&self, world: &WorldStore) -> bool {
        let chat = world.client_chat();
        self.player_list_display_revision == world.player_list_display_revision()
            && self.chat_delivery_revision == chat.delivery_revision
            && self.chat_display_refresh_revision == chat.display_refresh_revision
            && self.local_player == world.local_player_profile_id()
            && self.hidden_players.as_slice() == chat.presentation.hidden_players.as_slice()
            && self.blocked_players.as_slice() == chat.presentation.blocked_players.as_slice()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SocialInteractionsRuntime {
    generation: u64,
    page: SocialInteractionsPage,
    search: TextField,
    focus: HudSocialInteractionsFocus,
    hovered: Option<HudSocialInteractionsFocus>,
    hovered_since: Option<Instant>,
    rows: Vec<SocialPlayerState>,
    synced_revision: Option<SocialSyncRevision>,
    scroll_amount: f64,
    max_scroll_amount: f64,
    scrolling: bool,
    link_confirmation_open: bool,
}

impl SocialInteractionsRuntime {
    fn new(generation: u64, world: &WorldStore) -> Self {
        let mut state = Self {
            generation,
            page: SocialInteractionsPage::All,
            search: TextField::new(MAX_SEARCH_LENGTH),
            focus: HudSocialInteractionsFocus::Search,
            hovered: None,
            hovered_since: None,
            rows: Vec::new(),
            synced_revision: None,
            scroll_amount: 0.0,
            max_scroll_amount: 0.0,
            scrolling: false,
            link_confirmation_open: false,
        };
        state.rebuild_rows(world);
        state
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn page(&self) -> SocialInteractionsPage {
        self.page
    }

    pub(crate) fn search(&self) -> &TextField {
        &self.search
    }

    pub(super) fn search_mut(&mut self) -> &mut TextField {
        &mut self.search
    }

    pub(crate) fn focus(&self) -> HudSocialInteractionsFocus {
        self.focus
    }

    pub(crate) fn hovered(&self) -> Option<HudSocialInteractionsFocus> {
        self.hovered
    }

    pub(crate) fn tooltip_is_ready(&self, now: Instant) -> bool {
        matches!(
            (self.hovered, self.hovered_since),
            (
                Some(HudSocialInteractionsFocus::Player { .. }),
                Some(hovered_since)
            ) if now.saturating_duration_since(hovered_since) >= TOOLTIP_DELAY
        )
    }

    pub(crate) fn rows(&self) -> &[SocialPlayerState] {
        &self.rows
    }

    pub(crate) fn scroll_amount(&self) -> i32 {
        self.scroll_amount as i32
    }

    pub(crate) fn link_confirmation_is_open(&self) -> bool {
        self.link_confirmation_open
    }

    fn synchronize_rows(&mut self, world: &WorldStore) {
        if self
            .synced_revision
            .as_ref()
            .is_some_and(|revision| revision.matches(world))
        {
            return;
        }
        let incoming_players = world.social_players(SocialInteractionsPage::All);
        let incoming = incoming_players
            .iter()
            .enumerate()
            .map(|(index, player)| (player.profile.uuid, index))
            .collect::<HashMap<_, _>>();
        let mut existing_ids = HashSet::with_capacity(self.rows.len());

        for row in &mut self.rows {
            existing_ids.insert(row.profile.uuid);
            if let Some(&index) = incoming.get(&row.profile.uuid) {
                *row = incoming_players[index].clone();
            } else {
                row.removed = true;
                row.hidden = world.social_player_is_hidden(row.profile.uuid);
                row.blocked = world.social_player_is_blocked(row.profile.uuid);
            }
        }

        let search_lower = self.search.text().to_lowercase();
        for player in incoming_players {
            if existing_ids.contains(&player.profile.uuid)
                || !player_matches_live_page_add(&player, self.page)
                || !player_matches_search_lower(&player, &search_lower)
            {
                continue;
            }
            self.rows.push(player);
        }
        self.synced_revision = Some(SocialSyncRevision::from_world(world));
        self.refresh_player_action_focus(world);
    }

    fn rebuild_rows(&mut self, world: &WorldStore) {
        let search_lower = self.search.text().to_lowercase();
        self.rows = world
            .social_players(self.page)
            .into_iter()
            .filter(|player| player_matches_search_lower(player, &search_lower))
            .collect();
        self.synced_revision = Some(SocialSyncRevision::from_world(world));
        self.scroll_amount = self.scroll_amount.clamp(0.0, self.max_scroll_amount);
        self.validate_focus(world);
    }

    fn is_valid_focus(&self, world: &WorldStore, focus: HudSocialInteractionsFocus) -> bool {
        if self.link_confirmation_open {
            return matches!(
                focus,
                HudSocialInteractionsFocus::LinkOpen
                    | HudSocialInteractionsFocus::LinkCopy
                    | HudSocialInteractionsFocus::LinkCancel
            );
        }
        match focus {
            HudSocialInteractionsFocus::Search
            | HudSocialInteractionsFocus::Tab(_)
            | HudSocialInteractionsFocus::Done => true,
            HudSocialInteractionsFocus::BlockingHelp => {
                self.page == SocialInteractionsPage::Blocked
            }
            HudSocialInteractionsFocus::Player { profile_id, .. } => {
                let uuid = Uuid::from_bytes(profile_id);
                if Some(uuid) == world.local_player_profile_id() {
                    return false;
                }
                self.rows
                    .iter()
                    .find(|player| player.profile.uuid == uuid)
                    .is_some_and(|player| player_focus(player) == focus)
            }
            HudSocialInteractionsFocus::LinkOpen
            | HudSocialInteractionsFocus::LinkCopy
            | HudSocialInteractionsFocus::LinkCancel => false,
        }
    }

    fn set_page(&mut self, world: &WorldStore, page: SocialInteractionsPage) {
        self.page = page;
        self.focus = HudSocialInteractionsFocus::Tab(renderer_page(page));
        self.set_hovered(None);
        self.rebuild_rows(world);
    }

    fn focusables(&self, world: &WorldStore) -> Vec<HudSocialInteractionsFocus> {
        if self.link_confirmation_open {
            return vec![
                HudSocialInteractionsFocus::LinkOpen,
                HudSocialInteractionsFocus::LinkCopy,
                HudSocialInteractionsFocus::LinkCancel,
            ];
        }

        let mut focusables = vec![
            HudSocialInteractionsFocus::Search,
            HudSocialInteractionsFocus::Tab(HudSocialInteractionsPage::All),
            HudSocialInteractionsFocus::Tab(HudSocialInteractionsPage::Hidden),
            HudSocialInteractionsFocus::Tab(HudSocialInteractionsPage::Blocked),
        ];
        let local_player = world.local_player_profile_id();
        for player in &self.rows {
            if Some(player.profile.uuid) == local_player {
                continue;
            }
            focusables.push(player_focus(player));
        }
        if self.page == SocialInteractionsPage::Blocked {
            focusables.push(HudSocialInteractionsFocus::BlockingHelp);
        }
        focusables.push(HudSocialInteractionsFocus::Done);
        focusables
    }

    fn move_focus(&mut self, world: &WorldStore, forward: bool) {
        let focusables = self.focusables(world);
        let current = focusables
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % focusables.len()
        } else if current == 0 {
            focusables.len() - 1
        } else {
            current - 1
        };
        self.focus = focusables[next];
    }

    fn validate_focus(&mut self, world: &WorldStore) {
        if !self.is_valid_focus(world, self.focus) {
            self.focus = HudSocialInteractionsFocus::Search;
        }
    }

    fn refresh_player_action_focus(&mut self, world: &WorldStore) {
        self.focus = refreshed_player_focus(self.focus, &self.rows).unwrap_or(self.focus);
        let hovered = self
            .hovered
            .and_then(|hovered| refreshed_player_focus(hovered, &self.rows));
        self.set_hovered(hovered);
        self.validate_focus(world);
    }

    fn set_search_text_changed(&mut self, world: &WorldStore) {
        self.rebuild_rows(world);
        self.focus = HudSocialInteractionsFocus::Search;
        self.set_hovered(None);
    }

    fn set_hovered(&mut self, hovered: Option<HudSocialInteractionsFocus>) {
        if self.hovered != hovered {
            self.hovered = hovered;
            self.hovered_since = hovered.map(|_| bbb_client_clock::now());
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SocialInteractionsInputState {
    /// Tags HUD hit targets so clicks projected for an earlier screen miss.
    generation: u64,
    platform_requests: Vec<SocialInteractionsPlatformRequest>,
}

impl ClientInputState {
    pub(crate) fn social_interactions_runtime(&self) -> Option<&SocialInteractionsRuntime> {
        self.screen.social()
    }

    pub(crate) fn refresh_social_interactions_scroll_bounds(
        &mut self,
        surface_size: winit::dpi::PhysicalSize<u32>,
    ) {
        let Some(state) = self.screen.social_mut() else {
            return;
        };
        state.max_scroll_amount = f64::from(
            bbb_client_render::hud_social_interactions_max_scroll_amount(
                state.rows.len(),
                surface_size,
            ),
        );
        state.scroll_amount = state.scroll_amount.clamp(0.0, state.max_scroll_amount);
        if state.max_scroll_amount == 0.0 {
            state.scrolling = false;
        }
    }

    pub(crate) fn sync_social_interactions_screen(&mut self, world: &WorldStore) {
        if let Some(state) = self.screen.social_mut() {
            state.synchronize_rows(world);
        }
    }

    /// `key.socialInteractions` opens `SocialInteractionsScreen`.
    pub(crate) fn open_social_interactions_screen(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        world.hide_social_interactions_tutorial();
        self.interrupt_chat_screen();
        release_active_input(self, world, counters, net_commands);
        world.replace_screen();
        self.social_interactions.generation += 1;
        let screen = SocialInteractionsRuntime::new(self.social_interactions.generation, world);
        self.set_screen(Screen::Social(screen), counters, net_commands);
    }

    #[cfg(test)]
    pub(crate) fn handle_social_interactions_key(
        &mut self,
        world: &mut WorldStore,
        code: KeyCode,
        just_pressed: bool,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        self.handle_social_interactions_key_state(
            world,
            code,
            just_pressed,
            just_pressed,
            clipboard,
        )
    }

    pub(crate) fn handle_social_interactions_key_state(
        &mut self,
        world: &mut WorldStore,
        code: KeyCode,
        pressed: bool,
        just_pressed: bool,
        clipboard: &mut Option<&mut dyn DebugClipboard>,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        if self.screen.social().is_none() {
            return false;
        }
        if !pressed {
            return true;
        }

        let link_confirmation_open = self
            .screen
            .social()
            .is_some_and(|active| active.link_confirmation_open);
        if link_confirmation_open {
            if !just_pressed {
                return true;
            }
            match code {
                KeyCode::Escape => {
                    self.close_social_link_confirmation();
                }
                KeyCode::Tab => {
                    let forward = !self.shift_down();
                    self.screen
                        .social_mut()
                        .expect("active social link confirmation")
                        .move_focus(world, forward);
                }
                KeyCode::ArrowLeft | KeyCode::ArrowUp => {
                    self.screen
                        .social_mut()
                        .expect("active social link confirmation")
                        .move_focus(world, false);
                }
                KeyCode::ArrowRight | KeyCode::ArrowDown => {
                    self.screen
                        .social_mut()
                        .expect("active social link confirmation")
                        .move_focus(world, true);
                }
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space => {
                    self.activate_social_interactions_focus(world);
                }
                _ => {}
            }
            return true;
        }

        let search_focused = self.social_search_is_focused();
        if just_pressed
            && (code == KeyCode::Escape
                || (self.binding_matches("key.socialInteractions", code) && !search_focused))
        {
            self.close_social_interactions_screen();
            return true;
        }
        if code == KeyCode::Tab {
            let forward = !self.shift_down();
            self.screen
                .social_mut()
                .expect("active social screen")
                .move_focus(world, forward);
            return true;
        }
        if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown) && !search_focused {
            let forward = code == KeyCode::ArrowDown;
            self.screen
                .social_mut()
                .expect("active social screen")
                .move_focus(world, forward);
            return true;
        }
        if just_pressed
            && matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space)
            && !search_focused
        {
            self.activate_social_interactions_focus(world);
            return true;
        }

        if search_focused {
            let modifiers = self.text_field_modifiers();
            let state = self.screen.social_mut().expect("active social search");
            let before = state.search.value().to_vec();
            state.search.key(code, modifiers, clipboard, None);
            if state.search.value() != before {
                state.set_search_text_changed(world);
            }
        }
        true
    }

    /// Whether the social-interactions search box would take a commit right
    /// now, which is the same test `handle_social_interactions_text` makes
    /// before inserting.
    pub(crate) fn social_interactions_search_is_focused(&self) -> bool {
        self.screen.social().is_some_and(|state| {
            !state.link_confirmation_open && state.focus == HudSocialInteractionsFocus::Search
        })
    }

    pub(crate) fn handle_social_interactions_text(
        &mut self,
        world: &WorldStore,
        text: &str,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        let Some(state) = self.screen.social_mut() else {
            return false;
        };
        if state.link_confirmation_open || state.focus != HudSocialInteractionsFocus::Search {
            return true;
        }
        let before = state.search.value().to_vec();
        state.search.type_text(text, None);
        if state.search.value() != before {
            state.set_search_text_changed(world);
        }
        true
    }

    pub(crate) fn handle_social_interactions_hover(
        &mut self,
        world: &WorldStore,
        target: Option<HudSocialInteractionsHitTarget>,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        let Some(state) = self.screen.social_mut() else {
            return false;
        };
        let target = target.filter(|target| target.generation == state.generation);
        if state.scrolling {
            if let Some(HudSocialInteractionsHitTarget {
                kind: HudSocialInteractionsHitTargetKind::Scrollbar { scroll_amount },
                ..
            }) = target
            {
                state.scroll_amount = f64::from(scroll_amount).clamp(0.0, state.max_scroll_amount);
            }
        }
        let hovered = target.and_then(|target| match target.kind {
            HudSocialInteractionsHitTargetKind::Focus(focus)
                if state.is_valid_focus(world, focus) =>
            {
                Some(focus)
            }
            _ => None,
        });
        state.set_hovered(hovered);
        true
    }

    pub(crate) fn handle_social_interactions_mouse_input(
        &mut self,
        world: &mut WorldStore,
        button: MouseButton,
        element_state: ElementState,
        target: Option<HudSocialInteractionsHitTarget>,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        let Some(active) = self.screen.social() else {
            return false;
        };
        if button == MouseButton::Left && element_state == ElementState::Released {
            self.screen
                .social_mut()
                .expect("active social screen")
                .scrolling = false;
            return true;
        }
        if button != MouseButton::Left || element_state != ElementState::Pressed {
            return true;
        }
        let generation = active.generation;
        let Some(target) = target.filter(|target| target.generation == generation) else {
            return true;
        };
        match target.kind {
            HudSocialInteractionsHitTargetKind::Focus(focus) => {
                let valid = self
                    .screen
                    .social()
                    .is_some_and(|active| active.is_valid_focus(world, focus));
                if valid {
                    self.screen
                        .social_mut()
                        .expect("active social screen")
                        .focus = focus;
                    self.activate_social_interactions_focus(world);
                }
            }
            HudSocialInteractionsHitTargetKind::Scrollbar { scroll_amount } => {
                let state = self.screen.social_mut().expect("active social screen");
                if state.max_scroll_amount > 0.0 {
                    state.scrolling = true;
                    state.scroll_amount =
                        f64::from(scroll_amount).clamp(0.0, state.max_scroll_amount);
                }
            }
        }
        true
    }

    pub(crate) fn handle_social_interactions_mouse_wheel(
        &mut self,
        world: &WorldStore,
        delta: MouseScrollDelta,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        let Some(state) = self.screen.social_mut() else {
            return false;
        };
        let delta_pixels = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y) * f64::from((ROW_HEIGHT / 2).max(1)),
            MouseScrollDelta::PixelDelta(position) => position.y,
        };
        state.scroll_amount =
            (state.scroll_amount - delta_pixels).clamp(0.0, state.max_scroll_amount);
        true
    }

    pub(crate) fn take_social_interactions_platform_requests(
        &mut self,
    ) -> Vec<SocialInteractionsPlatformRequest> {
        std::mem::take(&mut self.social_interactions.platform_requests)
    }

    fn social_search_is_focused(&self) -> bool {
        self.screen
            .social()
            .is_some_and(|active| active.focus == HudSocialInteractionsFocus::Search)
    }

    fn close_social_interactions_screen(&mut self) {
        self.screen
            .close_if(|screen| matches!(screen, Screen::Social(_)));
    }

    fn activate_social_interactions_focus(&mut self, world: &mut WorldStore) {
        let Some(focus) = self.screen.social().map(|active| active.focus) else {
            return;
        };

        match focus {
            HudSocialInteractionsFocus::Search => {}
            HudSocialInteractionsFocus::Tab(page) => {
                self.screen
                    .social_mut()
                    .expect("active social screen")
                    .set_page(world, world_page(page));
            }
            HudSocialInteractionsFocus::Player { profile_id, action } => {
                let profile_id = Uuid::from_bytes(profile_id);
                if world.local_player_profile_id() == Some(profile_id) {
                    return;
                }
                let Some(player) = self.screen.social().and_then(|active| {
                    active
                        .rows
                        .iter()
                        .find(|player| player.profile.uuid == profile_id)
                }) else {
                    return;
                };
                let expected = if player.hidden {
                    HudSocialInteractionsPlayerAction::Show
                } else {
                    HudSocialInteractionsPlayerAction::Hide
                };
                if action != expected {
                    return;
                }
                match action {
                    HudSocialInteractionsPlayerAction::Hide => {
                        world.hide_social_player(profile_id);
                    }
                    HudSocialInteractionsPlayerAction::Show => {
                        world.show_social_player(profile_id);
                    }
                }
                let hidden = world.social_player_is_hidden(profile_id);
                let state = self.screen.social_mut().expect("active social screen");
                if let Some(player) = state
                    .rows
                    .iter_mut()
                    .find(|player| player.profile.uuid == profile_id)
                {
                    player.hidden = hidden;
                }
                state.synced_revision = Some(SocialSyncRevision::from_world(world));
                state.refresh_player_action_focus(world);
            }
            HudSocialInteractionsFocus::BlockingHelp => {
                let state = self.screen.social_mut().expect("active social screen");
                if state.page == SocialInteractionsPage::Blocked {
                    state.link_confirmation_open = true;
                    state.focus = HudSocialInteractionsFocus::LinkOpen;
                    state.set_hovered(None);
                }
            }
            HudSocialInteractionsFocus::Done => {
                self.close_social_interactions_screen();
            }
            HudSocialInteractionsFocus::LinkOpen => {
                self.social_interactions.platform_requests.push(
                    SocialInteractionsPlatformRequest::OpenUri(BLOCKING_HELP_URL.to_string()),
                );
                self.close_social_link_confirmation();
            }
            HudSocialInteractionsFocus::LinkCopy => {
                self.social_interactions.platform_requests.push(
                    SocialInteractionsPlatformRequest::ClipboardWrite(
                        BLOCKING_HELP_URL.to_string(),
                    ),
                );
                self.close_social_link_confirmation();
            }
            HudSocialInteractionsFocus::LinkCancel => {
                self.close_social_link_confirmation();
            }
        }
    }

    fn close_social_link_confirmation(&mut self) {
        let Some(state) = self.screen.social_mut() else {
            return;
        };
        state.link_confirmation_open = false;
        state.focus = HudSocialInteractionsFocus::BlockingHelp;
        state.set_hovered(None);
    }
}

fn player_matches_live_page_add(player: &SocialPlayerState, page: SocialInteractionsPage) -> bool {
    match page {
        SocialInteractionsPage::All => true,
        SocialInteractionsPage::Hidden | SocialInteractionsPage::Blocked => {
            !player.removed && (player.hidden || player.blocked)
        }
    }
}

fn player_matches_search_lower(player: &SocialPlayerState, search_lower: &str) -> bool {
    search_lower.is_empty() || player.profile.name.to_lowercase().contains(search_lower)
}

fn player_focus(player: &SocialPlayerState) -> HudSocialInteractionsFocus {
    HudSocialInteractionsFocus::Player {
        profile_id: *player.profile.uuid.as_bytes(),
        action: if player.hidden {
            HudSocialInteractionsPlayerAction::Show
        } else {
            HudSocialInteractionsPlayerAction::Hide
        },
    }
}

fn refreshed_player_focus(
    focus: HudSocialInteractionsFocus,
    rows: &[SocialPlayerState],
) -> Option<HudSocialInteractionsFocus> {
    let HudSocialInteractionsFocus::Player { profile_id, .. } = focus else {
        return Some(focus);
    };
    let profile_id = Uuid::from_bytes(profile_id);
    rows.iter()
        .find(|player| player.profile.uuid == profile_id)
        .map(player_focus)
}

fn renderer_page(page: SocialInteractionsPage) -> HudSocialInteractionsPage {
    match page {
        SocialInteractionsPage::All => HudSocialInteractionsPage::All,
        SocialInteractionsPage::Hidden => HudSocialInteractionsPage::Hidden,
        SocialInteractionsPage::Blocked => HudSocialInteractionsPage::Blocked,
    }
}

fn world_page(page: HudSocialInteractionsPage) -> SocialInteractionsPage {
    match page {
        HudSocialInteractionsPage::All => SocialInteractionsPage::All,
        HudSocialInteractionsPage::Hidden => SocialInteractionsPage::Hidden,
        HudSocialInteractionsPage::Blocked => SocialInteractionsPage::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use crate::control::NetCounters;
    use bbb_client_world::ChatPresentationConfig;
    use bbb_protocol::packets::{
        DialogHolder, GameProfile, GameProfileProperty, GameType, PlayerInfoAction,
        PlayerInfoEntry, PlayerInfoRemove, PlayerInfoUpdate, ShowDialog,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use winit::keyboard::PhysicalKey;

    use super::*;
    use crate::input::handle_key_input;

    #[test]
    fn social_interactions_search_mouse_actions_and_links_are_generation_safe() {
        let mut world = WorldStore::new();
        let local_id = Uuid::from_u128(0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa);
        let ada_id = Uuid::parse_str("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb").unwrap();
        world.set_local_player_profile_id(local_id);
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![
                player_info_entry(local_id, "Zulu"),
                player_info_entry(ada_id, "Ada"),
            ],
        });
        let mut config = ChatPresentationConfig::default();
        config.blocked_players.push(ada_id);
        world.set_chat_presentation_config(config);

        let mut input = ClientInputState::new(true);
        open_social(&mut input, &mut world);
        assert!(input.handle_social_interactions_text(&world, "ad"));
        let active = input.social_interactions_runtime().unwrap();
        assert_eq!(active.search.text(), "ad");
        assert_eq!(active.rows.len(), 1);
        assert_eq!(active.rows[0].profile.uuid, ada_id);
        let generation = active.generation;

        assert!(input.handle_social_interactions_key(&mut world, KeyCode::KeyP, true, &mut None));
        assert!(input.social_interactions_runtime().is_some());
        let hide = HudSocialInteractionsFocus::Player {
            profile_id: *ada_id.as_bytes(),
            action: HudSocialInteractionsPlayerAction::Hide,
        };
        assert!(input.handle_social_interactions_mouse_input(
            &mut world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudSocialInteractionsHitTarget {
                generation: generation + 1,
                kind: HudSocialInteractionsHitTargetKind::Focus(hide),
            }),
        ));
        assert!(!world.social_player_is_hidden(ada_id));
        assert!(input.handle_social_interactions_mouse_input(
            &mut world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudSocialInteractionsHitTarget {
                generation,
                kind: HudSocialInteractionsHitTargetKind::Focus(hide),
            }),
        ));
        assert!(world.social_player_is_hidden(ada_id));
        assert_eq!(
            input.social_interactions_runtime().unwrap().focus,
            HudSocialInteractionsFocus::Player {
                profile_id: *ada_id.as_bytes(),
                action: HudSocialInteractionsPlayerAction::Show,
            }
        );

        click_focus(
            &mut input,
            &mut world,
            generation,
            HudSocialInteractionsFocus::Tab(HudSocialInteractionsPage::Blocked),
        );
        let active = input.social_interactions_runtime().unwrap();
        assert_eq!(active.page, SocialInteractionsPage::Blocked);
        assert_eq!(active.rows.len(), 1);
        click_focus(
            &mut input,
            &mut world,
            generation,
            HudSocialInteractionsFocus::BlockingHelp,
        );
        assert!(
            input
                .social_interactions_runtime()
                .unwrap()
                .link_confirmation_open
        );
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Enter, true, &mut None));
        assert_eq!(
            input.take_social_interactions_platform_requests(),
            vec![SocialInteractionsPlatformRequest::OpenUri(
                BLOCKING_HELP_URL.to_string()
            )]
        );
        assert!(input
            .take_social_interactions_platform_requests()
            .is_empty());

        click_focus(
            &mut input,
            &mut world,
            generation,
            HudSocialInteractionsFocus::BlockingHelp,
        );
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Tab, true, &mut None));
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Enter, true, &mut None));
        assert_eq!(
            input.take_social_interactions_platform_requests(),
            vec![SocialInteractionsPlatformRequest::ClipboardWrite(
                BLOCKING_HELP_URL.to_string()
            )]
        );

        {
            let active = input.screen.social_mut().unwrap();
            active.max_scroll_amount = 90.0;
        }
        assert!(input.handle_social_interactions_mouse_wheel(
            &world,
            MouseScrollDelta::LineDelta(0.0, -1.0),
        ));
        assert_eq!(
            input.social_interactions_runtime().unwrap().scroll_amount,
            18.0
        );
        assert!(input.handle_social_interactions_mouse_input(
            &mut world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudSocialInteractionsHitTarget {
                generation,
                kind: HudSocialInteractionsHitTargetKind::Scrollbar { scroll_amount: 80 },
            }),
        ));
        assert_eq!(
            input.social_interactions_runtime().unwrap().scroll_amount,
            80.0
        );

        click_focus(
            &mut input,
            &mut world,
            generation,
            HudSocialInteractionsFocus::Search,
        );
        assert!(
            input.handle_social_interactions_text(&world, "0123456789abcdefghijklmnopqrstuvwxyz",)
        );
        assert_eq!(
            input
                .social_interactions_runtime()
                .unwrap()
                .search
                .text()
                .chars()
                .count(),
            MAX_SEARCH_LENGTH
        );

        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Escape, true, &mut None));
        assert!(input.social_interactions_runtime().is_none());
    }

    #[test]
    fn social_interactions_player_tooltip_uses_vanilla_half_second_delay() {
        let mut world = WorldStore::new();
        let player_id = Uuid::from_u128(0x11111111111141118111111111111111);
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![player_info_entry(player_id, "Hovered")],
        });
        let mut input = ClientInputState::new(true);
        open_social(&mut input, &mut world);
        let generation = input.social_interactions_runtime().unwrap().generation;
        let target = HudSocialInteractionsHitTarget {
            generation,
            kind: HudSocialInteractionsHitTargetKind::Focus(HudSocialInteractionsFocus::Player {
                profile_id: *player_id.as_bytes(),
                action: HudSocialInteractionsPlayerAction::Hide,
            }),
        };

        assert!(input.handle_social_interactions_hover(&world, Some(target)));
        let runtime = input.social_interactions_runtime().unwrap();
        let hovered_since = runtime.hovered_since.expect("hover start");
        assert!(!runtime.tooltip_is_ready(hovered_since + Duration::from_millis(499)));
        assert!(runtime.tooltip_is_ready(hovered_since + Duration::from_millis(500)));

        assert!(input.handle_social_interactions_hover(&world, Some(target)));
        assert_eq!(
            input.social_interactions_runtime().unwrap().hovered_since,
            Some(hovered_since),
            "steady hover must not restart the tooltip delay"
        );
        assert!(input.handle_social_interactions_hover(&world, None));
        assert_eq!(
            input.social_interactions_runtime().unwrap().hovered_since,
            None
        );
    }

    #[test]
    fn player_info_remove_marks_retained_social_row_and_readd_restores_it() {
        let mut world = WorldStore::new();
        let player_id = Uuid::from_u128(0x11111111111141118111111111111111);
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![player_info_entry(player_id, "Transient")],
        });
        let mut input = ClientInputState::new(true);
        open_social(&mut input, &mut world);
        let generation = input.social_interactions_runtime().unwrap().generation;

        world.apply_player_info_remove(PlayerInfoRemove {
            profile_ids: vec![player_id],
        });
        input.sync_social_interactions_screen(&world);
        let retained = &input.social_interactions_runtime().unwrap().rows[0];
        assert_eq!(retained.profile.uuid, player_id);
        assert!(retained.removed);

        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![player_info_entry(player_id, "Transient")],
        });
        input.sync_social_interactions_screen(&world);
        assert!(!input.social_interactions_runtime().unwrap().rows[0].removed);

        world.apply_player_info_remove(PlayerInfoRemove {
            profile_ids: vec![player_id],
        });
        input.sync_social_interactions_screen(&world);
        click_focus(
            &mut input,
            &mut world,
            generation,
            HudSocialInteractionsFocus::Tab(HudSocialInteractionsPage::Hidden),
        );
        assert!(input.social_interactions_runtime().unwrap().rows.is_empty());
        click_focus(
            &mut input,
            &mut world,
            generation,
            HudSocialInteractionsFocus::Tab(HudSocialInteractionsPage::All),
        );
        assert!(input.social_interactions_runtime().unwrap().rows.is_empty());
    }

    #[test]
    fn social_interactions_default_p_key_opens_and_closes_away_from_search() {
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        world.apply_login(&bbb_protocol::packets::PlayLogin {
            player_id: 7,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 0,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        });
        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_LEVEL_CHUNKS_LOAD_START_ID,
            param: 0.0,
        });
        world.complete_client_level_load(false);
        assert!(world.show_social_interactions_tutorial_if_first_join());

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyP),
            ElementState::Pressed,
        );
        assert!(input.social_interactions_runtime().is_some());
        assert_eq!(
            world
                .client_toast_events()
                .back()
                .and_then(|event| event.tutorial()),
            Some(bbb_client_world::ClientTutorialToastEvent::Hide {
                token: bbb_client_world::ClientTutorialToastToken::SocialInteractions,
            })
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyP),
            ElementState::Released,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::Tab),
            ElementState::Pressed,
        );
        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyP),
            ElementState::Pressed,
        );
        assert!(input.social_interactions_runtime().is_none());
    }

    #[test]
    fn social_interactions_dialog_restores_same_native_input_runtime() {
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        open_social(&mut input, &mut world);
        input.handle_social_interactions_text(&world, "retained");
        let generation = input.social_interactions_runtime().unwrap().generation;

        world.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::string("minecraft:notice"),
                }]),
            },
        });
        input.apply_screen_requests(&mut world, &mut counters, &None);
        assert!(input.social_interactions_runtime().is_none());

        world.apply_clear_dialog();
        input.apply_screen_requests(&mut world, &mut counters, &None);
        let restored = input.social_interactions_runtime().unwrap();
        assert_eq!(restored.generation, generation);
        assert_eq!(restored.search.text(), "retained");
    }

    #[test]
    fn social_interactions_search_supports_clipboard_key_repeat_and_rebound_close_key() {
        struct StubClipboard(String);
        impl DebugClipboard for StubClipboard {
            fn set_debug_clipboard_text(&mut self, text: &str) -> bool {
                self.0 = text.to_string();
                true
            }
            fn get_debug_clipboard_text(&mut self) -> Option<String> {
                Some(self.0.clone())
            }
        }

        let mut input = ClientInputState::new(true);
        let mut world = WorldStore::new();
        open_social(&mut input, &mut world);
        let mut clip = StubClipboard("Alex".to_string());
        let mut clip_ref: Option<&mut dyn DebugClipboard> = Some(&mut clip);

        // Paste with shortcut modifier (Cmd on macOS, Ctrl elsewhere).
        let mod_key = if cfg!(target_os = "macos") {
            KeyCode::SuperLeft
        } else {
            KeyCode::ControlLeft
        };
        input.set_physical_key_down(mod_key, true);
        input.set_control_key(mod_key, true);
        assert!(input.handle_social_interactions_key_state(
            &mut world,
            KeyCode::KeyV,
            true,
            true,
            &mut clip_ref,
        ));
        input.set_physical_key_down(mod_key, false);
        input.set_control_key(mod_key, false);
        assert_eq!(
            input.social_interactions_runtime().unwrap().search.text(),
            "Alex"
        );

        // Key repeat (pressed=true, just_pressed=false) deletes characters in search.
        assert!(input.handle_social_interactions_key_state(
            &mut world,
            KeyCode::Backspace,
            true,
            false,
            &mut clip_ref,
        ));
        assert_eq!(
            input.social_interactions_runtime().unwrap().search.text(),
            "Ale"
        );

        // Rebound key.socialInteractions closes when search is not focused, while KeyP does not.
        let mut options = crate::options::ClientOptions::defaults(Default::default());
        options.set_dynamic_raw_for_test("key_key.socialInteractions", "key.keyboard.k");
        input.set_key_bindings(crate::options::key_bindings::KeyBindings::from_options(
            &options,
        ));
        assert!(input.handle_social_interactions_key_state(
            &mut world,
            KeyCode::Tab,
            true,
            true,
            &mut clip_ref,
        ));
        assert!(input.handle_social_interactions_key_state(
            &mut world,
            KeyCode::KeyP,
            true,
            true,
            &mut clip_ref,
        ));
        assert!(input.social_interactions_runtime().is_some());
        assert!(input.handle_social_interactions_key_state(
            &mut world,
            KeyCode::KeyK,
            true,
            true,
            &mut clip_ref,
        ));
        assert!(input.social_interactions_runtime().is_none());
    }

    fn open_social(input: &mut ClientInputState, world: &mut WorldStore) {
        input.open_social_interactions_screen(world, &mut NetCounters::default(), &None);
    }

    fn click_focus(
        input: &mut ClientInputState,
        world: &mut WorldStore,
        generation: u64,
        focus: HudSocialInteractionsFocus,
    ) {
        assert!(input.handle_social_interactions_mouse_input(
            world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudSocialInteractionsHitTarget {
                generation,
                kind: HudSocialInteractionsHitTargetKind::Focus(focus),
            }),
        ));
    }

    fn player_info_entry(profile_id: Uuid, name: &str) -> PlayerInfoEntry {
        PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: name.to_string(),
                properties: vec![GameProfileProperty {
                    name: "textures".to_string(),
                    value: "skin-payload".to_string(),
                    signature: Some("skin-signature".to_string()),
                }],
            }),
            listed: true,
            latency: 0,
            game_mode: GameType::default(),
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }
    }
}
