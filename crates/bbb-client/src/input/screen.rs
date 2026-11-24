//! `Minecraft.screen`: the one open screen, each kind carrying the state it
//! owns while open, above the parents it returns to (`lastScreen`).

use bbb_client_net::NetCommand;
use bbb_client_world::{ScreenRequest, WorldStore};
use bbb_protocol::packets::SeenAdvancements;
use tokio::sync::mpsc;

use super::{
    advancements_screen::AdvancementsScreen,
    book::BookScreen,
    death_screen::DeathScreen,
    disconnected_screen::DisconnectedScreenRuntime,
    game_event_screen::{sample_easter_egg_logo, GameEventScreenRuntime},
    game_rules::GameRulesScreen,
    inventory::{creative::CreativeInventoryInputState, ContainerInputState},
    jigsaw_block::JigsawBlockEditorInputState,
    maybe_queue_command_suggestion_request,
    pause_screen::DebugPauseScreenState,
    queue_seen_advancements_command, queue_sign_editor_removed, release_active_input,
    sign_editor_input_state,
    social::SocialInteractionsRuntime,
    stats_screen::StatsScreen,
    structure_block::StructureBlockEditorInputState,
    test_block::TestBlockEditorInputState,
    test_instance_block::TestInstanceBlockEditorInputState,
    ChatEntryState, ChatRestrictionsScreenState, ChatScreenExitReason, ClientInputState,
    CommandEntryKind, DebugGameModeSwitcherState, DebugOptionsScreenState, SignEditorInputState,
};
use crate::control::NetCounters;

#[derive(Debug, Clone, PartialEq)]
// Several active editor/dialog variants hold inline text-field state on the single UI screen stack.
#[allow(clippy::large_enum_variant)]
pub(crate) enum Screen {
    Dialog(super::dialog::DialogScreenState),
    DialogWarning(Option<bool>),
    DialogLink(super::dialog::DialogLinkConfirmationState),
    DialogWaiting(u32),
    CommandConfirmation(super::command_confirmation::CommandConfirmation),
    Container(ContainerInputState),
    Creative(CreativeInventoryInputState),
    /// `PauseScreen`, including its feedback subscreen page.
    Pause(DebugPauseScreenState),
    DebugOptions(DebugOptionsScreenState),
    /// `GameModeSwitcherScreen` (F3+F4).
    GameModeSwitcher(DebugGameModeSwitcherState),
    Disconnected(DisconnectedScreenRuntime),
    CommandEntry(ChatEntryState),
    /// Opened over the chat screen it returns to.
    ChatRestrictions(ChatRestrictionsScreenState),
    /// `SignEditScreen` / `HangingSignEditScreen`.
    Sign(SignEditorInputState),
    Death(DeathScreen),
    /// `WinScreen` or the demo intro screen.
    GameEvent(GameEventScreenRuntime),
    Advancements(AdvancementsScreen),
    Stats(StatsScreen),
    Social(SocialInteractionsRuntime),
    /// `InGameGameRulesScreen`.
    GameRules(GameRulesScreen),
    Book(BookScreen),
    StructureBlock(StructureBlockEditorInputState),
    JigsawBlock(JigsawBlockEditorInputState),
    TestBlock(TestBlockEditorInputState),
    TestInstanceBlock(TestInstanceBlockEditorInputState),
}

impl Screen {
    /// Mirrors Vanilla `Screen.isAllowedInPortal` (`Screen.java:438-440`) and
    /// screen overrides (`ChatScreen`, `DeathScreen`, `WinScreen`, `PauseScreen`).
    pub(crate) fn is_allowed_in_portal(&self) -> bool {
        matches!(
            self,
            Self::Pause(_)
                | Self::CommandEntry(ChatEntryState {
                    owner: CommandEntryKind::Chat(_),
                    ..
                })
                | Self::Death(_)
                | Self::GameEvent(_)
                | Self::Disconnected(_)
                | Self::ChatRestrictions(_)
                | Self::CommandConfirmation(_)
        )
    }
}

/// The current screen is the last entry; each entry below it is the screen
/// the one above returns to when it closes.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Screens {
    stack: Vec<Screen>,
}

impl Screens {
    pub(crate) fn current_mut(&mut self) -> Option<&mut Screen> {
        self.stack.last_mut()
    }
    pub(crate) fn remove_if(&mut self, mut predicate: impl FnMut(&Screen) -> bool) -> Vec<Screen> {
        let mut removed = Vec::new();
        let mut i = 0;
        while i < self.stack.len() {
            if predicate(&self.stack[i]) {
                removed.push(self.stack.remove(i));
            } else {
                i += 1;
            }
        }
        removed
    }
    pub(crate) fn current(&self) -> Option<&Screen> {
        self.stack.last()
    }

    pub(crate) fn is_open(&self) -> bool {
        !self.stack.is_empty()
    }

    /// The screen the current one returns to.
    pub(crate) fn parent(&self) -> Option<&Screen> {
        self.stack.iter().rev().nth(1)
    }

    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &Screen> {
        self.stack.iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Screen> {
        self.stack.iter_mut()
    }

    pub(crate) fn into_screens(self) -> std::vec::IntoIter<Screen> {
        self.stack.into_iter()
    }

    /// Opens `screen` with the current screen as the one it returns to.
    pub(crate) fn open_child(&mut self, screen: Screen) {
        self.stack.push(screen);
    }

    /// `Minecraft.setScreen`: replaces the current screen and every parent.
    /// Only `set_screen` calls it, after the replaced screens ran `removed`.
    fn set(&mut self, screen: Screen) {
        self.stack.clear();
        self.stack.push(screen);
    }

    /// `setScreen(lastScreen)`: closes the current screen, returning to its
    /// parent if it has one.
    pub(crate) fn close(&mut self) -> Option<Screen> {
        self.stack.pop()
    }

    /// Closes the current screen when `is_kind` accepts it.
    pub(crate) fn close_if(&mut self, is_kind: impl FnOnce(&Screen) -> bool) -> Option<Screen> {
        if self.current().is_some_and(is_kind) {
            self.close()
        } else {
            None
        }
    }

    pub(crate) fn take_chat_entry(&mut self) -> Option<ChatEntryState> {
        match self.close_if(|screen| matches!(screen, Screen::CommandEntry(_))) {
            Some(Screen::CommandEntry(entry)) => Some(entry),
            _ => None,
        }
    }
}

impl ClientInputState {
    pub(crate) fn blocks_world_input(&self) -> bool {
        self.screen.current().is_some_and(|screen| {
            !matches!(
                screen,
                Screen::GameModeSwitcher(_)
                    | Screen::CommandEntry(ChatEntryState {
                        owner: CommandEntryKind::Chat(_),
                        ..
                    })
            )
        })
    }

    pub(crate) fn wants_cursor(&self) -> bool {
        self.screen.current().is_some_and(|screen| {
            !matches!(
                screen,
                Screen::CommandEntry(ChatEntryState {
                    owner: CommandEntryKind::Chat(_),
                    ..
                })
            )
        })
    }

    /// Applies the packet listener's `Minecraft.setScreen` calls in packet order.
    pub(crate) fn apply_screen_requests(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.apply_screen_requests_with_resources(world, counters, net_commands, None);
    }

    pub(crate) fn refresh_resource_pack_prompt(
        &mut self,
        world: &WorldStore,
        item_runtime: Option<&bbb_client_render::item_model::NativeItemRuntime>,
    ) {
        let prompt = world.resource_pack_prompt(|component| match item_runtime {
            Some(runtime) => bbb_client_assets::localized_component_runs(
                component,
                &Default::default(),
                &crate::component_text::ClientTextLookup(Some(runtime)),
            ),
            None => bbb_protocol::styled_component_runs_from_nbt_value(component),
        });
        if let Some(prompt) = prompt {
            self.show_dialog(prompt);
        } else if self
            .current_dialog()
            .is_some_and(|dialog| dialog.is_resource_pack_prompt())
        {
            self.close_dialog();
        }
    }

    pub(crate) fn apply_screen_requests_with_resources(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
        item_runtime: Option<&bbb_client_render::item_model::NativeItemRuntime>,
    ) {
        let requests = world.take_screen_requests();
        let mut iter = requests.into_iter().peekable();
        while let Some(request) = iter.next() {
            match request {
                ScreenRequest::ShowDialog(dialog) => {
                    if !self.dialog_screen_is_open() {
                        if self.screen.chat_entry().is_some() {
                            self.reset_chat_scroll();
                        }
                        if let Some(editor) = self.screen.sign_editor_mut() {
                            editor.ime_preedit = None;
                            queue_sign_editor_removed(counters, net_commands, editor);
                        }
                        if let Some(advancements) = self.screen.advancements_mut() {
                            // AdvancementsScreen.removed.
                            advancements.mouse_left_down = false;
                            advancements.is_scrolling = false;
                            queue_seen_advancements_command(
                                counters,
                                net_commands,
                                SeenAdvancements::ClosedScreen,
                            );
                        }
                    }
                    self.show_dialog(dialog);
                }
                ScreenRequest::RefreshResourcePackPrompt => {
                    while matches!(iter.peek(), Some(ScreenRequest::RefreshResourcePackPrompt)) {
                        let _ = iter.next();
                    }
                    self.refresh_resource_pack_prompt(world, item_runtime);
                }
                ScreenRequest::ClearDialog => {
                    if self.close_dialog() {
                        self.refresh_dialog_parent_suggestions(world, counters, net_commands);
                    }
                }
                ScreenRequest::CommandBlockEditor(editor) => {
                    self.open_command_block_editor(editor, world, counters, net_commands);
                }
                ScreenRequest::CommandBlockEditorData(editor) => {
                    self.refresh_command_block_editor(editor);
                    maybe_queue_command_suggestion_request(self, counters, world, net_commands);
                }
                ScreenRequest::CommandBlockEditorMode { pos, mode } => {
                    self.refresh_command_block_editor_mode(pos, mode, world);
                    maybe_queue_command_suggestion_request(self, counters, world, net_commands);
                }
                ScreenRequest::CloseCommandBlockEditorInChunk(chunk) => {
                    self.close_command_entry_if(|kind| {
                        matches!(kind,
                        CommandEntryKind::CommandBlock(editor) if editor.pos.chunk() == chunk)
                    });
                }
                ScreenRequest::MinecartCommandBlockEditor(editor) => {
                    self.open_minecart_command_block_editor(editor, world, counters, net_commands);
                }
                ScreenRequest::MinecartCommandBlockEditorData(editor) => {
                    self.refresh_minecart_command_block_editor(editor);
                }
                ScreenRequest::CloseMinecartCommandBlockEditor(entity_id) => {
                    self.close_command_entry_if(|kind| matches!(kind,
                        CommandEntryKind::MinecartCommandBlock(editor) if entity_id == editor.entity_id));
                }
                ScreenRequest::SignEditor(request) => {
                    let editor = sign_editor_input_state(self, world, &request);
                    self.set_screen(Screen::Sign(editor), counters, net_commands);
                }
                ScreenRequest::Book(book) => {
                    self.set_screen(Screen::Book(BookScreen::new(book)), counters, net_commands)
                }
                ScreenRequest::Death(screen) => self.set_screen(
                    Screen::Death(DeathScreen::new(screen)),
                    counters,
                    net_commands,
                ),
                ScreenRequest::GameEvent(screen) => {
                    let logo = matches!(
                        screen,
                        bbb_client_world::ClientGameEventScreenState::Win { .. }
                    ) && sample_easter_egg_logo();
                    let screen = GameEventScreenRuntime::new(screen, logo);
                    self.set_screen(Screen::GameEvent(screen), counters, net_commands);
                }
                ScreenRequest::CloseDeathScreen => {
                    let is_death = |screen: &Screen| matches!(screen, Screen::Death(_));
                    if self.screen.current().is_some_and(is_death) {
                        self.close_screens(counters, net_commands);
                    }
                    self.screen.remove_if(is_death);
                }
                ScreenRequest::CloseLevelScreens => {
                    let level_bound = |screen: &Screen| {
                        matches!(
                            screen,
                            Screen::CommandEntry(ChatEntryState {
                                owner: CommandEntryKind::CommandBlock(_)
                                    | CommandEntryKind::MinecartCommandBlock(_),
                                ..
                            }) | Screen::Death(_)
                                | Screen::GameEvent(_)
                                | Screen::Social(_)
                                | Screen::GameRules(_)
                                | Screen::StructureBlock(_)
                                | Screen::JigsawBlock(_)
                                | Screen::TestBlock(_)
                                | Screen::TestInstanceBlock(_)
                                | Screen::Container(_)
                                | Screen::Creative(_)
                        )
                    };
                    let dialog_current = self.dialog_screen_is_open();
                    let confirmation_current = self.command_confirmation().is_some();
                    if confirmation_current {
                        self.screen.close();
                        if let Some(chat) = self.screen.take_chat_entry() {
                            self.finalize_chat_entry(chat, ChatScreenExitReason::Interrupted);
                        }
                    }
                    if dialog_current || confirmation_current {
                        for screen in self.screen.remove_if(level_bound) {
                            match screen {
                                Screen::CommandEntry(entry) => self
                                    .finalize_chat_entry(entry, ChatScreenExitReason::Interrupted),
                                screen => self.remember_inventory_preferences(screen),
                            }
                        }
                    } else if self.screen.iter().any(level_bound) {
                        self.close_screens(counters, net_commands);
                    }
                    self.creative_selected_tab = None;
                    self.creative_drop_spam_count = 0;
                }
                ScreenRequest::Container => {
                    self.open_inventory_screen(world, None, counters, net_commands);
                    self.refresh_creative_visible_items(world, None);
                }
                ScreenRequest::CreativeInventory(operator_tab) => {
                    self.open_inventory_screen(world, Some(operator_tab), counters, net_commands);
                    self.refresh_creative_visible_items(world, None);
                }
                ScreenRequest::CloseScreen => self.close_screens(counters, net_commands),
                ScreenRequest::CloseContainer => {
                    let is_container = |screen: &Screen| {
                        matches!(screen, Screen::Container(_) | Screen::Creative(_))
                    };
                    if self.screen.current().is_some_and(is_container) {
                        self.close_screens(counters, net_commands);
                    }
                    self.close_retained_inventory();
                }
                ScreenRequest::RefreshCreativeInventory => {
                    self.refresh_creative_inventory(world);
                    self.refresh_creative_visible_items(world, None);
                }
                ScreenRequest::ReconcileCreativeInventory => {
                    if self.creative_inventory_context().is_some()
                        && !world
                            .local_player()
                            .abilities
                            .as_ref()
                            .is_some_and(|abilities| abilities.instabuild)
                    {
                        world.replace_screen();
                        world.open_local_inventory();
                        self.apply_screen_requests(world, counters, net_commands);
                    }
                }
                ScreenRequest::GameRuleValues => {
                    if let (Some(screen), Some(values)) =
                        (self.screen.game_rules_mut(), world.last_game_rule_values())
                    {
                        screen.rules.load(values);
                    }
                }
                ScreenRequest::CloseGameRules => {
                    self.close_game_rules_screen();
                    self.screen
                        .remove_if(|screen| matches!(screen, Screen::GameRules(_)));
                }
                ScreenRequest::TestInstanceBlockStatus(status) => {
                    self.show_test_instance_block_status(world, status)
                }
                ScreenRequest::StatsUpdated => {
                    for screen in self.screen.iter_mut() {
                        if let Screen::Stats(stats) = screen {
                            stats.loading = false;
                        }
                    }
                }
            }
        }
    }

    pub(super) fn command_entries_mut(&mut self) -> impl Iterator<Item = &mut ChatEntryState> {
        self.screen.iter_mut().filter_map(|screen| match screen {
            Screen::CommandEntry(entry) => Some(entry),
            _ => None,
        })
    }
    pub(super) fn close_command_entry_if(
        &mut self,
        matches_kind: impl Fn(&CommandEntryKind) -> bool,
    ) {
        self.screen.remove_if(
            |screen| matches!(screen, Screen::CommandEntry(entry) if matches_kind(&entry.owner)),
        );
    }
    /// `Minecraft.setScreen`: the screen it replaces runs `Screen.removed`.
    pub(crate) fn set_screen(
        &mut self,
        screen: Screen,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.close_screens(counters, net_commands);
        self.dialog_screen_revision = self.dialog_screen_revision.saturating_add(1);
        self.screen.set(screen);
    }

    /// An editor opened by using its block or entity: held input stops and
    /// the screens the world owns close.
    pub(crate) fn open_editor_screen(
        &mut self,
        screen: Screen,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        release_active_input(self, world, counters, net_commands);
        world.replace_screen();
        self.set_screen(screen, counters, net_commands);
    }

    /// Mirrors `LocalPlayer.handlePortalTransitionEffect` (`LocalPlayer.java:943-949`):
    /// closes any open screen with `!isAllowedInPortal()`, sending `ContainerClose`
    /// when a container screen is open.
    pub(crate) fn handle_portal_transition_screen_close(
        &mut self,
        world: &mut WorldStore,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        if self
            .screen
            .current()
            .is_some_and(|screen| !screen.is_allowed_in_portal())
        {
            if matches!(
                self.screen.current(),
                Some(Screen::Container(_) | Screen::Creative(_))
            ) {
                super::queue_container_close_command(counters, world, net_commands);
            } else {
                world.replace_screen();
            }
            self.close_screens(counters, net_commands);
        } else if self.screen.current().is_none() {
            super::queue_container_close_command(counters, world, net_commands);
        }
        self.apply_screen_requests(world, counters, net_commands);
    }

    /// `setScreen(null)` with every parent dropped.
    pub(crate) fn close_screens(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        self.close_retained_inventory();
        // Parents already ran `removed` when the screen above them opened.
        for (index, screen) in std::mem::take(&mut self.screen)
            .into_screens()
            .rev()
            .enumerate()
        {
            match screen {
                screen @ (Screen::Container(_) | Screen::Creative(_)) => {
                    self.remember_inventory_preferences(screen)
                }
                Screen::CommandEntry(entry) => {
                    self.finalize_chat_entry(entry, ChatScreenExitReason::Interrupted)
                }
                Screen::Sign(editor) if index == 0 => {
                    queue_sign_editor_removed(counters, net_commands, &editor)
                }
                Screen::Advancements(_) if index == 0 => queue_seen_advancements_command(
                    counters,
                    net_commands,
                    SeenAdvancements::ClosedScreen,
                ),
                _ => {}
            }
        }
    }
}

macro_rules! screen_variants {
    ($($variant:ident($state:ty) => $get:ident $(, $get_mut:ident)?;)*) => {
        impl Screens {
            $(
                pub(crate) fn $get(&self) -> Option<&$state> {
                    match self.stack.last() {
                        Some(Screen::$variant(state)) => Some(state),
                        _ => None,
                    }
                }

                $(
                    pub(crate) fn $get_mut(&mut self) -> Option<&mut $state> {
                        match self.stack.last_mut() {
                            Some(Screen::$variant(state)) => Some(state),
                            _ => None,
                        }
                    }
                )?
            )*
        }
    };
}

screen_variants! {
    Dialog(super::dialog::DialogScreenState) => dialog;
    DialogLink(super::dialog::DialogLinkConfirmationState) => dialog_link, dialog_link_mut;
    CommandConfirmation(super::command_confirmation::CommandConfirmation) => command_confirmation, command_confirmation_mut;
    Container(ContainerInputState) => container, container_mut;
    Creative(CreativeInventoryInputState) => creative, creative_mut;
    Pause(DebugPauseScreenState) => pause, pause_mut;
    DebugOptions(DebugOptionsScreenState) => debug_options, debug_options_mut;
    GameModeSwitcher(DebugGameModeSwitcherState) => game_mode_switcher, game_mode_switcher_mut;
    Disconnected(DisconnectedScreenRuntime) => disconnected, disconnected_mut;
    CommandEntry(ChatEntryState) => chat_entry, chat_entry_mut;
    ChatRestrictions(ChatRestrictionsScreenState) => chat_restrictions;
    Sign(SignEditorInputState) => sign_editor, sign_editor_mut;
    Death(DeathScreen) => death, death_mut;
    GameEvent(GameEventScreenRuntime) => game_event, game_event_mut;
    Advancements(AdvancementsScreen) => advancements, advancements_mut;
    Stats(StatsScreen) => stats, stats_mut;
    Social(SocialInteractionsRuntime) => social, social_mut;
    GameRules(GameRulesScreen) => game_rules, game_rules_mut;
    Book(BookScreen) => book, book_mut;
    StructureBlock(StructureBlockEditorInputState) => structure_block, structure_block_mut;
    JigsawBlock(JigsawBlockEditorInputState) => jigsaw_block, jigsaw_block_mut;
    TestBlock(TestBlockEditorInputState) => test_block, test_block_mut;
    TestInstanceBlock(TestInstanceBlockEditorInputState) => test_instance_block, test_instance_block_mut;
}

#[cfg(test)]
impl ClientInputState {
    pub(in crate::input) fn container_for_test() -> Self {
        let mut input = Self::new(true);
        input.set_screen_for_test(Screen::Container(Default::default()));
        input
    }

    pub(crate) fn set_screen_for_test(&mut self, screen: Screen) {
        self.screen.set(screen);
    }
}
