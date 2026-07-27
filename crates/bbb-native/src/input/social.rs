use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use bbb_renderer::{
    HudSocialInteractionsFocus, HudSocialInteractionsHitTarget, HudSocialInteractionsHitTargetKind,
    HudSocialInteractionsPage, HudSocialInteractionsPlayerAction,
};
use bbb_world::{SocialInteractionsPage, SocialPlayerState, WorldStore};
use uuid::Uuid;
use winit::{
    event::{ElementState, MouseButton, MouseScrollDelta},
    keyboard::KeyCode,
};

use super::{text_edit, ClientInputState};

pub(crate) const BLOCKING_HELP_URL: &str = "https://aka.ms/javablocking";
const MAX_SEARCH_CHARS: usize = 16;
const ROW_HEIGHT: i32 = 36;
const TOOLTIP_DELAY: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SocialInteractionsPlatformRequest {
    OpenUri(String),
    ClipboardWrite(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SocialInteractionsRuntime {
    generation: u64,
    return_to_pause: bool,
    page: SocialInteractionsPage,
    search_text: String,
    search_cursor: usize,
    search_selection: usize,
    focus: HudSocialInteractionsFocus,
    hovered: Option<HudSocialInteractionsFocus>,
    hovered_since: Option<Instant>,
    rows: Vec<SocialPlayerState>,
    scroll_amount: f64,
    max_scroll_amount: f64,
    scrolling: bool,
    link_confirmation_open: bool,
}

impl SocialInteractionsRuntime {
    fn new(generation: u64, return_to_pause: bool, world: &WorldStore) -> Self {
        let mut state = Self {
            generation,
            return_to_pause,
            page: SocialInteractionsPage::All,
            search_text: String::new(),
            search_cursor: 0,
            search_selection: 0,
            focus: HudSocialInteractionsFocus::Search,
            hovered: None,
            hovered_since: None,
            rows: Vec::new(),
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

    pub(crate) fn search_text(&self) -> &str {
        &self.search_text
    }

    pub(crate) fn search_cursor(&self) -> usize {
        self.search_cursor
    }

    pub(crate) fn search_selection(&self) -> usize {
        self.search_selection
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
        let incoming = world
            .social_players(SocialInteractionsPage::All)
            .into_iter()
            .map(|player| (player.profile.uuid, player))
            .collect::<BTreeMap<_, _>>();

        for row in &mut self.rows {
            if let Some(current) = incoming.get(&row.profile.uuid) {
                *row = current.clone();
            } else {
                row.removed = true;
                row.hidden = world.social_player_is_hidden(row.profile.uuid);
                row.blocked = world.social_player_is_blocked(row.profile.uuid);
            }
        }

        for player in incoming.into_values() {
            if self
                .rows
                .iter()
                .any(|row| row.profile.uuid == player.profile.uuid)
                || !player_matches_live_page_add(&player, self.page)
                || !player_matches_search(&player, &self.search_text)
            {
                continue;
            }
            self.rows.push(player);
        }
        self.refresh_player_action_focus(world);
    }

    fn rebuild_rows(&mut self, world: &WorldStore) {
        self.rows = world
            .social_players(self.page)
            .into_iter()
            .filter(|player| player_matches_search(player, &self.search_text))
            .collect();
        self.scroll_amount = self.scroll_amount.clamp(0.0, self.max_scroll_amount);
        self.validate_focus(world);
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
        focusables.extend(self.rows.iter().filter_map(|player| {
            (Some(player.profile.uuid) != local_player).then(|| player_focus(player))
        }));
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
        if !self.focusables(world).contains(&self.focus) {
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
            self.hovered_since = hovered.map(|_| Instant::now());
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SocialInteractionsInputState {
    active: Option<SocialInteractionsRuntime>,
    platform_requests: Vec<SocialInteractionsPlatformRequest>,
}

impl ClientInputState {
    pub(crate) fn social_interactions_runtime(&self) -> Option<&SocialInteractionsRuntime> {
        self.social_interactions.active.as_ref()
    }

    pub(crate) fn refresh_social_interactions_scroll_bounds(
        &mut self,
        surface_size: winit::dpi::PhysicalSize<u32>,
    ) {
        let Some(state) = self.social_interactions.active.as_mut() else {
            return;
        };
        state.max_scroll_amount = f64::from(
            bbb_renderer::hud_social_interactions_max_scroll_amount(state.rows.len(), surface_size),
        );
        state.scroll_amount = state.scroll_amount.clamp(0.0, state.max_scroll_amount);
        if state.max_scroll_amount == 0.0 {
            state.scrolling = false;
        }
    }

    pub(super) fn take_social_interactions_for_dialog(
        &mut self,
    ) -> Option<SocialInteractionsRuntime> {
        self.social_interactions.active.take()
    }

    pub(super) fn restore_social_interactions_after_dialog(
        &mut self,
        world: &WorldStore,
        state: SocialInteractionsRuntime,
    ) {
        if world
            .social_interactions_screen()
            .is_some_and(|screen| screen.generation == state.generation)
        {
            self.social_interactions.active = Some(state);
        }
    }

    pub(crate) fn sync_social_interactions_screen(&mut self, world: &WorldStore) -> bool {
        let generation = world
            .social_interactions_screen()
            .map(|screen| screen.generation);
        if generation.is_none() {
            return self.social_interactions.active.take().is_some();
        }
        if self
            .social_interactions
            .active
            .as_ref()
            .is_some_and(|active| Some(active.generation) == generation)
        {
            self.social_interactions
                .active
                .as_mut()
                .expect("same-generation social runtime exists")
                .synchronize_rows(world);
            return false;
        }
        self.social_interactions.active =
            generation.map(|generation| SocialInteractionsRuntime::new(generation, false, world));
        true
    }

    pub(crate) fn open_social_interactions_screen(
        &mut self,
        world: &mut WorldStore,
        return_to_pause: bool,
    ) -> bool {
        if world.social_interactions_screen().is_some() {
            self.sync_social_interactions_screen(world);
            return false;
        }
        if return_to_pause {
            self.close_debug_pause_screen();
        }
        let screen = world.open_social_interactions_screen();
        self.social_interactions.active = Some(SocialInteractionsRuntime::new(
            screen.generation,
            return_to_pause,
            world,
        ));
        true
    }

    pub(crate) fn handle_social_interactions_key(
        &mut self,
        world: &mut WorldStore,
        code: KeyCode,
        just_pressed: bool,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        if self.social_interactions.active.is_none() {
            return false;
        }
        if !just_pressed {
            return true;
        }

        let link_confirmation_open = self
            .social_interactions
            .active
            .as_ref()
            .is_some_and(|active| active.link_confirmation_open);
        if link_confirmation_open {
            match code {
                KeyCode::Escape => {
                    self.close_social_link_confirmation();
                }
                KeyCode::Tab => {
                    let forward = !self.shift_down();
                    self.social_interactions
                        .active
                        .as_mut()
                        .expect("active social link confirmation")
                        .move_focus(world, forward);
                }
                KeyCode::ArrowLeft | KeyCode::ArrowUp => {
                    self.social_interactions
                        .active
                        .as_mut()
                        .expect("active social link confirmation")
                        .move_focus(world, false);
                }
                KeyCode::ArrowRight | KeyCode::ArrowDown => {
                    self.social_interactions
                        .active
                        .as_mut()
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
        if code == KeyCode::Escape || (code == KeyCode::KeyP && !search_focused) {
            self.close_social_interactions_screen(world);
            return true;
        }
        if code == KeyCode::Tab {
            let forward = !self.shift_down();
            self.social_interactions
                .active
                .as_mut()
                .expect("active social screen")
                .move_focus(world, forward);
            return true;
        }
        if matches!(code, KeyCode::ArrowUp | KeyCode::ArrowDown) && !search_focused {
            let forward = code == KeyCode::ArrowDown;
            self.social_interactions
                .active
                .as_mut()
                .expect("active social screen")
                .move_focus(world, forward);
            return true;
        }
        if matches!(code, KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Space) && !search_focused
        {
            self.activate_social_interactions_focus(world);
            return true;
        }

        if search_focused {
            let shift_down = self.shift_down();
            let control_down = self.control_down();
            let changed = {
                let state = self
                    .social_interactions
                    .active
                    .as_mut()
                    .expect("active social search");
                edit_search_key(
                    &mut state.search_text,
                    &mut state.search_cursor,
                    &mut state.search_selection,
                    code,
                    shift_down,
                    control_down,
                )
            };
            if changed {
                self.social_interactions
                    .active
                    .as_mut()
                    .expect("active social search")
                    .set_search_text_changed(world);
            }
        }
        true
    }

    pub(crate) fn handle_social_interactions_text(
        &mut self,
        world: &WorldStore,
        text: &str,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        let Some(state) = self.social_interactions.active.as_mut() else {
            return false;
        };
        if state.link_confirmation_open
            || state.focus != HudSocialInteractionsFocus::Search
            || !insert_search_text(
                &mut state.search_text,
                &mut state.search_cursor,
                &mut state.search_selection,
                text,
            )
        {
            return true;
        }
        state.set_search_text_changed(world);
        true
    }

    pub(crate) fn handle_social_interactions_hover(
        &mut self,
        world: &WorldStore,
        target: Option<HudSocialInteractionsHitTarget>,
    ) -> bool {
        self.sync_social_interactions_screen(world);
        let Some(state) = self.social_interactions.active.as_mut() else {
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
                if state.focusables(world).contains(&focus) =>
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
        let Some(active) = self.social_interactions.active.as_ref() else {
            return false;
        };
        if button == MouseButton::Left && element_state == ElementState::Released {
            self.social_interactions
                .active
                .as_mut()
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
                    .social_interactions
                    .active
                    .as_ref()
                    .is_some_and(|active| active.focusables(world).contains(&focus));
                if valid {
                    self.social_interactions
                        .active
                        .as_mut()
                        .expect("active social screen")
                        .focus = focus;
                    self.activate_social_interactions_focus(world);
                }
            }
            HudSocialInteractionsHitTargetKind::Scrollbar { scroll_amount } => {
                let state = self
                    .social_interactions
                    .active
                    .as_mut()
                    .expect("active social screen");
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
        let Some(state) = self.social_interactions.active.as_mut() else {
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
        self.social_interactions
            .active
            .as_ref()
            .is_some_and(|active| active.focus == HudSocialInteractionsFocus::Search)
    }

    fn close_social_interactions_screen(&mut self, world: &mut WorldStore) {
        let Some(active) = self.social_interactions.active.take() else {
            return;
        };
        if !world.close_social_interactions_screen(active.generation) {
            return;
        }
        if active.return_to_pause {
            self.open_debug_pause_screen_with_menu(world);
        }
    }

    fn activate_social_interactions_focus(&mut self, world: &mut WorldStore) {
        let Some((generation, focus)) = self
            .social_interactions
            .active
            .as_ref()
            .map(|active| (active.generation, active.focus))
        else {
            return;
        };
        if world
            .social_interactions_screen()
            .is_none_or(|screen| screen.generation != generation)
        {
            self.social_interactions.active = None;
            return;
        }

        match focus {
            HudSocialInteractionsFocus::Search => {}
            HudSocialInteractionsFocus::Tab(page) => {
                self.social_interactions
                    .active
                    .as_mut()
                    .expect("active social screen")
                    .set_page(world, world_page(page));
            }
            HudSocialInteractionsFocus::Player { profile_id, action } => {
                let profile_id = Uuid::from_bytes(profile_id);
                if world.local_player_profile_id() == Some(profile_id) {
                    return;
                }
                let Some(player) = self.social_interactions.active.as_ref().and_then(|active| {
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
                let state = self
                    .social_interactions
                    .active
                    .as_mut()
                    .expect("active social screen");
                if let Some(player) = state
                    .rows
                    .iter_mut()
                    .find(|player| player.profile.uuid == profile_id)
                {
                    player.hidden = hidden;
                }
                state.refresh_player_action_focus(world);
            }
            HudSocialInteractionsFocus::BlockingHelp => {
                let state = self
                    .social_interactions
                    .active
                    .as_mut()
                    .expect("active social screen");
                if state.page == SocialInteractionsPage::Blocked {
                    state.link_confirmation_open = true;
                    state.focus = HudSocialInteractionsFocus::LinkOpen;
                    state.set_hovered(None);
                }
            }
            HudSocialInteractionsFocus::Done => {
                self.close_social_interactions_screen(world);
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
        let Some(state) = self.social_interactions.active.as_mut() else {
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

fn player_matches_search(player: &SocialPlayerState, search: &str) -> bool {
    search.is_empty()
        || player
            .profile
            .name
            .to_lowercase()
            .contains(&search.to_lowercase())
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

fn edit_search_key(
    text: &mut String,
    cursor: &mut usize,
    selection: &mut usize,
    code: KeyCode,
    shift_down: bool,
    control_down: bool,
) -> bool {
    match code {
        KeyCode::KeyA if control_down => {
            *cursor = text_edit::char_len(text);
            *selection = 0;
            false
        }
        KeyCode::ArrowLeft => {
            let next = if control_down {
                text_edit::word_position(text, *cursor, -1)
            } else {
                cursor.saturating_sub(1)
            };
            set_cursor(cursor, selection, next, shift_down, text);
            false
        }
        KeyCode::ArrowRight => {
            let next = if control_down {
                text_edit::word_position(text, *cursor, 1)
            } else {
                cursor.saturating_add(1)
            };
            set_cursor(cursor, selection, next, shift_down, text);
            false
        }
        KeyCode::Home => {
            set_cursor(cursor, selection, 0, shift_down, text);
            false
        }
        KeyCode::End => {
            set_cursor(
                cursor,
                selection,
                text_edit::char_len(text),
                shift_down,
                text,
            );
            false
        }
        KeyCode::Backspace => {
            if delete_selection(text, cursor, selection) {
                return true;
            }
            if control_down {
                text_edit::remove_word_before_cursor(text, cursor);
            } else if *cursor > 0 {
                let start = text_edit::byte_index(text, *cursor - 1);
                let end = text_edit::byte_index(text, *cursor);
                text.replace_range(start..end, "");
                *cursor -= 1;
            }
            *selection = *cursor;
            true
        }
        KeyCode::Delete => {
            if delete_selection(text, cursor, selection) {
                return true;
            }
            if control_down {
                text_edit::remove_word_at_cursor(text, *cursor);
            } else if *cursor < text_edit::char_len(text) {
                let start = text_edit::byte_index(text, *cursor);
                let end = text_edit::byte_index(text, *cursor + 1);
                text.replace_range(start..end, "");
            }
            *selection = *cursor;
            true
        }
        _ => false,
    }
}

fn insert_search_text(
    text: &mut String,
    cursor: &mut usize,
    selection: &mut usize,
    inserted: &str,
) -> bool {
    let inserted = inserted
        .chars()
        .filter(|ch| !ch.is_control())
        .take(MAX_SEARCH_CHARS)
        .collect::<String>();
    if inserted.is_empty() {
        return false;
    }
    delete_selection(text, cursor, selection);
    let remaining = MAX_SEARCH_CHARS.saturating_sub(text_edit::char_len(text));
    let inserted = inserted.chars().take(remaining).collect::<String>();
    if inserted.is_empty() {
        return false;
    }
    let byte = text_edit::byte_index(text, *cursor);
    text.insert_str(byte, &inserted);
    *cursor += text_edit::char_len(&inserted);
    *selection = *cursor;
    true
}

fn delete_selection(text: &mut String, cursor: &mut usize, selection: &mut usize) -> bool {
    if *cursor == *selection {
        return false;
    }
    let start = (*cursor).min(*selection);
    let end = (*cursor).max(*selection);
    let start_byte = text_edit::byte_index(text, start);
    let end_byte = text_edit::byte_index(text, end);
    text.replace_range(start_byte..end_byte, "");
    *cursor = start;
    *selection = start;
    true
}

fn set_cursor(cursor: &mut usize, selection: &mut usize, next: usize, selecting: bool, text: &str) {
    *cursor = next.min(text_edit::char_len(text));
    if !selecting {
        *selection = *cursor;
    }
}

#[cfg(test)]
mod tests {
    use bbb_control::NetCounters;
    use bbb_protocol::packets::{
        DialogHolder, GameProfile, GameProfileProperty, GameType, PlayerInfoAction,
        PlayerInfoEntry, PlayerInfoRemove, PlayerInfoUpdate, ShowDialog,
    };
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
    use bbb_world::ChatPresentationConfig;
    use winit::keyboard::PhysicalKey;

    use super::*;
    use crate::input::handle_key_input;

    #[test]
    fn social_interactions_search_mouse_actions_links_and_pause_return_are_generation_safe() {
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
        assert!(input.open_social_interactions_screen(&mut world, false));
        assert!(input.handle_social_interactions_text(&world, "ad"));
        let active = input.social_interactions_runtime().unwrap();
        assert_eq!(active.search_text, "ad");
        assert_eq!(active.rows.len(), 1);
        assert_eq!(active.rows[0].profile.uuid, ada_id);
        let generation = active.generation;

        assert!(input.handle_social_interactions_key(&mut world, KeyCode::KeyP, true));
        assert!(world.social_interactions_screen().is_some());
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
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Enter, true));
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
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Tab, true));
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Enter, true));
        assert_eq!(
            input.take_social_interactions_platform_requests(),
            vec![SocialInteractionsPlatformRequest::ClipboardWrite(
                BLOCKING_HELP_URL.to_string()
            )]
        );

        {
            let active = input.social_interactions.active.as_mut().unwrap();
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
            text_edit::char_len(&input.social_interactions_runtime().unwrap().search_text),
            MAX_SEARCH_CHARS
        );

        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Escape, true));
        assert!(world.social_interactions_screen().is_none());
        assert!(!input.debug_pause_screen_is_open());

        input.open_debug_pause_screen_with_menu(&world);
        assert!(input.open_social_interactions_screen(&mut world, true));
        assert!(!input.debug_pause_screen_is_open());
        assert!(input.handle_social_interactions_key(&mut world, KeyCode::Escape, true));
        assert!(world.social_interactions_screen().is_none());
        assert!(input.debug_pause_screen_is_open());
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
        input.open_social_interactions_screen(&mut world, false);
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
        input.open_social_interactions_screen(&mut world, false);
        let generation = input.social_interactions_runtime().unwrap().generation;

        world.apply_player_info_remove(PlayerInfoRemove {
            profile_ids: vec![player_id],
        });
        assert!(!input.sync_social_interactions_screen(&world));
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

        handle_key_input(
            &mut input,
            &mut counters,
            &mut world,
            &None,
            PhysicalKey::Code(KeyCode::KeyP),
            ElementState::Pressed,
        );
        assert!(world.social_interactions_screen().is_some());
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
        assert!(world.social_interactions_screen().is_none());
    }

    #[test]
    fn social_interactions_dialog_restores_same_native_input_runtime() {
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = WorldStore::new();
        input.open_social_interactions_screen(&mut world, true);
        input.handle_social_interactions_text(&world, "retained");
        let generation = input.social_interactions_runtime().unwrap().generation;

        world.apply_show_dialog(ShowDialog {
            dialog: DialogHolder::Direct {
                dialog: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "type".to_string(),
                    value: NbtSummaryValue::String("minecraft:notice".to_string()),
                }]),
            },
        });
        input.sync_dialog_native_previous_screen(&world, &mut counters, &None);
        input.sync_social_interactions_screen(&world);
        assert!(input.social_interactions_runtime().is_none());

        world.apply_clear_dialog();
        input.sync_dialog_native_previous_screen(&world, &mut counters, &None);
        input.sync_social_interactions_screen(&world);
        let restored = input.social_interactions_runtime().unwrap();
        assert_eq!(restored.generation, generation);
        assert_eq!(restored.search_text, "retained");
        assert!(restored.return_to_pause);
    }

    #[test]
    fn social_interactions_screen_narration_schedules_mouse_and_keyboard_inputs() {
        let commands = include_str!("../simulation/input_commands.rs");
        let social_effects = commands
            .match_indices("PlatformEffect::SocialInteractionsInput")
            .count();
        assert!(social_effects >= 6);
        assert!(commands.contains(
            "PlatformEffect::SocialInteractionsInput(\n                    \
             ScreenNarrationScheduleInput::MouseMove"
        ));
        assert!(commands.contains(
            "PlatformEffect::SocialInteractionsInput(\n                    \
             ScreenNarrationScheduleInput::MouseAction"
        ));
        assert!(commands.contains(
            "PlatformEffect::SocialInteractionsInput(\n                    \
             ScreenNarrationScheduleInput::KeyboardAction"
        ));

        let platform = include_str!("../main.rs");
        assert!(platform.contains("self.social_interactions_narration"));
        assert!(platform.contains(".after_input(input, Instant::now());"));
        assert!(platform.contains("social_interactions_narration.tick("));
        assert!(platform.contains("hud_render_runtime.social_interactions_narration()"));
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
