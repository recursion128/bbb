use super::*;

#[cfg(test)]
pub(super) const DEBUG_OPTIONS_HEADER_HEIGHT: i32 = 61;
#[cfg(not(test))]
const DEBUG_OPTIONS_HEADER_HEIGHT: i32 = 61;
const DEBUG_OPTIONS_FOOTER_HEIGHT: i32 = 33;
#[cfg(test)]
pub(super) const DEBUG_OPTIONS_ROW_WIDTH: i32 = 350;
#[cfg(not(test))]
const DEBUG_OPTIONS_ROW_WIDTH: i32 = 350;
#[cfg(test)]
pub(super) const DEBUG_OPTIONS_ROW_HEIGHT: i32 = 20;
#[cfg(not(test))]
const DEBUG_OPTIONS_ROW_HEIGHT: i32 = 20;
#[cfg(test)]
pub(super) const DEBUG_OPTIONS_STATUS_BUTTON_WIDTH: i32 = 60;
#[cfg(not(test))]
const DEBUG_OPTIONS_STATUS_BUTTON_WIDTH: i32 = 60;
const DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH: i32 = 120;
const DEBUG_OPTIONS_DONE_BUTTON_WIDTH: i32 = 60;
const DEBUG_OPTIONS_SCROLLBAR_WIDTH: i32 = 6;
const DEBUG_OPTIONS_SCROLLBAR_MIN_HEIGHT: i32 = 32;
const DEBUG_OPTIONS_SEARCH_WIDTH: i32 = DEBUG_OPTIONS_ROW_WIDTH / 3;
#[cfg(test)]
pub(super) const DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET: i32 = 4;
#[cfg(not(test))]
const DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET: i32 = 4;
const DEBUG_OPTIONS_SEARCH_INNER_WIDTH: i32 = DEBUG_OPTIONS_SEARCH_WIDTH - 8;
#[cfg(test)]
pub(super) const DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);
#[cfg(not(test))]
const DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD: Duration = Duration::from_millis(250);
pub(super) const DEBUG_GAME_MODE_SWITCHER_SLOT_AREA: i32 = 26;
pub(super) const DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED: i32 = 31;
pub(super) const DEBUG_GAME_MODE_SWITCHER_ALL_SLOTS_WIDTH: i32 =
    4 * DEBUG_GAME_MODE_SWITCHER_SLOT_PADDED - 5;
pub(super) const DEBUG_GAME_MODE_SWITCHER_SLOT_Y_OFFSET: i32 = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DebugGameModeSwitcherState {
    pub(super) selected: GameType,
    pub(super) first_mouse_position: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DebugOptionsScreenFocus {
    #[default]
    Search,
    List,
    Entry {
        entry: DebugScreenEntryId,
        status: DebugScreenEntryStatus,
    },
    DefaultProfile,
    PerformanceProfile,
    Done,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenState {
    pub(super) search: text_field::TextField,
    pub(super) search_selecting: bool,
    pub(super) last_left_click_at: Option<Instant>,
    pub(super) scroll_row: usize,
    pub(super) scrollbar_drag_y: Option<i32>,
    pub(super) cursor_position: Option<(i32, i32)>,
    pub(super) focus: DebugOptionsScreenFocus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenHudState {
    pub(crate) search: text_field::TextField,
    pub(crate) search_focused: bool,
    pub(crate) rows: Vec<DebugOptionsScreenHudRow>,
    pub(crate) tooltip: Option<DebugOptionsScreenTooltip>,
    pub(crate) scroll_row: usize,
    pub(crate) total_rows: usize,
    pub(crate) visible_rows: usize,
    pub(crate) default_profile_active: bool,
    pub(crate) default_profile_hovered: bool,
    pub(crate) default_profile_focused: bool,
    pub(crate) performance_profile_active: bool,
    pub(crate) performance_profile_hovered: bool,
    pub(crate) performance_profile_focused: bool,
    pub(crate) done_hovered: bool,
    pub(crate) done_focused: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DebugOptionsScreenHudRow {
    Category {
        label: String,
    },
    Entry {
        entry: DebugScreenEntryId,
        path: String,
        status: DebugScreenEntryStatus,
        hovered_status: Option<DebugScreenEntryStatus>,
        focused_status: Option<DebugScreenEntryStatus>,
        allowed: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DebugOptionsScreenTooltip {
    pub(crate) text: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DebugOptionsScreenRow {
    Category { label: &'static str },
    Entry(DebugScreenEntryId),
}

impl ClientInputState {
    pub(crate) fn open_debug_options_screen(
        &mut self,
        counters: &mut NetCounters,
        net_commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let screen = Screen::DebugOptions(DebugOptionsScreenState::default());
        self.set_screen(screen, counters, net_commands);
    }

    pub(crate) fn close_debug_options_screen(&mut self) {
        self.screen
            .close_if(|screen| matches!(screen, Screen::DebugOptions(_)));
    }

    pub(crate) fn debug_options_screen_is_open(&self) -> bool {
        self.screen.debug_options().is_some()
    }

    pub(crate) fn debug_options_screen_search_text(&self) -> Option<String> {
        self.screen
            .debug_options()
            .map(|screen| screen.search.text())
    }

    pub(crate) fn debug_options_screen_focus(&self) -> Option<DebugOptionsScreenFocus> {
        self.screen.debug_options().map(|screen| screen.focus)
    }

    pub(crate) fn debug_options_screen_hovered_row(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> Option<(usize, DebugOptionsScreenRow)> {
        let screen = self.screen.debug_options()?;
        let (mouse_x, mouse_y) = screen.cursor_position?;
        let visible_row = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
        let rows = debug_options_screen_rows(&screen.search.text());
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(rows.len(), visible_rows));
        rows.get(scroll_row + visible_row)
            .copied()
            .map(|row| (scroll_row + visible_row, row))
    }

    pub(crate) fn debug_options_screen_search_hovered(
        &self,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some((mouse_x, mouse_y)) = self
            .screen
            .debug_options()
            .and_then(|screen| screen.cursor_position)
        else {
            return false;
        };
        let (x, y, width, height) = debug_options_search_box_rect(surface_size);
        mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
    }

    pub(crate) fn debug_options_screen_hud_state(
        &self,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
    ) -> Option<DebugOptionsScreenHudState> {
        let screen = self.screen.debug_options()?;
        let rows = debug_options_screen_rows(&screen.search.text());
        let total_rows = rows.len();
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(total_rows, visible_rows));
        let hovered_status_button = screen.cursor_position.and_then(|(mouse_x, mouse_y)| {
            self.debug_options_status_button_at(mouse_x, mouse_y, surface_size, reduced_debug_info)
        });
        let hovered_profile_button = screen.cursor_position.and_then(|(mouse_x, mouse_y)| {
            debug_options_profile_button_at(mouse_x, mouse_y, surface_size)
        });
        let done_hovered = screen.cursor_position.is_some_and(|(mouse_x, mouse_y)| {
            debug_options_done_button_contains(mouse_x, mouse_y, surface_size)
        });
        let rows = rows
            .into_iter()
            .skip(scroll_row)
            .take(visible_rows)
            .map(|row| match row {
                DebugOptionsScreenRow::Category { label } => DebugOptionsScreenHudRow::Category {
                    label: label.to_string(),
                },
                DebugOptionsScreenRow::Entry(entry) => DebugOptionsScreenHudRow::Entry {
                    entry,
                    path: entry.path().to_string(),
                    status: self.debug_entries.status(entry),
                    hovered_status: hovered_status_button
                        .filter(|(hovered_entry, _)| *hovered_entry == entry)
                        .map(|(_, status)| status),
                    focused_status: match screen.focus {
                        DebugOptionsScreenFocus::Entry {
                            entry: focused_entry,
                            status,
                        } if focused_entry == entry => Some(status),
                        _ => None,
                    },
                    allowed: entry.is_allowed(reduced_debug_info),
                },
            })
            .collect();
        let tooltip = screen
            .cursor_position
            .and_then(|(mouse_x, mouse_y)| {
                debug_options_not_allowed_tooltip_at(
                    &screen.search.text(),
                    scroll_row,
                    mouse_x,
                    mouse_y,
                    surface_size,
                    reduced_debug_info,
                )
            })
            .map(|(x, y)| DebugOptionsScreenTooltip {
                text: "Not visible when debug info is reduced".to_string(),
                x,
                y,
            });
        Some(DebugOptionsScreenHudState {
            search: screen.search.clone(),
            search_focused: screen.focus == DebugOptionsScreenFocus::Search,
            rows,
            tooltip,
            scroll_row,
            total_rows,
            visible_rows,
            default_profile_active: !self
                .debug_entries
                .is_using_profile(DebugScreenProfile::Default),
            default_profile_hovered: hovered_profile_button == Some(DebugScreenProfile::Default),
            default_profile_focused: screen.focus == DebugOptionsScreenFocus::DefaultProfile,
            performance_profile_active: !self
                .debug_entries
                .is_using_profile(DebugScreenProfile::Performance),
            performance_profile_hovered: hovered_profile_button
                == Some(DebugScreenProfile::Performance),
            performance_profile_focused: screen.focus
                == DebugOptionsScreenFocus::PerformanceProfile,
            done_hovered,
            done_focused: screen.focus == DebugOptionsScreenFocus::Done,
        })
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_key(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
    ) -> bool {
        self.handle_debug_options_screen_key_with_clipboard(physical_key, state, None)
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_key_with_clipboard(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        clipboard: Option<&mut dyn DebugClipboard>,
    ) -> bool {
        self.handle_debug_options_screen_key_with_clipboard_and_surface(
            physical_key,
            state,
            clipboard,
            PhysicalSize::new(800, 600),
        )
    }

    pub(crate) fn handle_debug_options_screen_key_with_clipboard_and_surface(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        mut clipboard: Option<&mut dyn DebugClipboard>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        if self.screen.debug_options().is_none() {
            return false;
        }
        let PhysicalKey::Code(code) = physical_key else {
            return true;
        };
        let pressed = matches!(state, ElementState::Pressed);
        if matches!(code, KeyCode::ShiftLeft | KeyCode::ShiftRight) {
            self.set_shift_key(code, pressed);
        }
        if matches!(code, KeyCode::ControlLeft | KeyCode::ControlRight) {
            self.set_control_key(code, pressed);
        }
        if matches!(code, KeyCode::SuperLeft | KeyCode::SuperRight) {
            self.set_key_down(code, pressed);
        }
        if matches!(code, KeyCode::AltLeft | KeyCode::AltRight) {
            self.set_key_down(code, pressed);
        }
        if matches!(code, KeyCode::F3) || self.debug_modifier_down {
            return false;
        }
        if !pressed {
            return true;
        }
        let search_focused = self
            .screen
            .debug_options()
            .is_some_and(|screen| screen.focus == DebugOptionsScreenFocus::Search);
        match code {
            KeyCode::Escape => {
                self.set_key_down(code, false);
                self.close_debug_options_screen();
            }
            KeyCode::Tab => {
                self.move_debug_options_tab_focus(self.shift_down(), surface_size);
            }
            KeyCode::Enter | KeyCode::Space if !search_focused => {
                self.activate_debug_options_focus();
            }
            KeyCode::ArrowUp if !search_focused => {
                self.move_debug_options_vertical_focus(-1, surface_size);
            }
            KeyCode::ArrowDown => {
                self.move_debug_options_vertical_focus(1, surface_size);
            }
            KeyCode::ArrowLeft if !search_focused => {
                self.move_debug_options_horizontal_focus(-1, surface_size);
            }
            KeyCode::ArrowRight if !search_focused => {
                self.move_debug_options_horizontal_focus(1, surface_size);
            }
            _ if search_focused => {
                let modifiers = self.text_field_modifiers();
                if let Some(screen) = self.screen.debug_options_mut() {
                    let before = screen.search.value().to_vec();
                    screen.search.key(code, modifiers, &mut clipboard, None);
                    if screen.search.value() != before {
                        screen.scroll_row = 0;
                    }
                }
            }
            _ => {}
        }
        true
    }

    fn move_debug_options_tab_focus(&mut self, backwards: bool, surface_size: PhysicalSize<u32>) {
        let Some(screen) = self.screen.debug_options() else {
            return;
        };
        let current = screen.focus;
        let mut order = vec![(DebugOptionsScreenFocus::Search, true)];
        for row in debug_options_screen_rows(&screen.search.text()) {
            let DebugOptionsScreenRow::Entry(entry) = row else {
                continue;
            };
            let current_status = self.debug_entries.status(entry);
            for status in [
                DebugScreenEntryStatus::Never,
                DebugScreenEntryStatus::InOverlay,
                DebugScreenEntryStatus::AlwaysOn,
            ] {
                order.push((
                    DebugOptionsScreenFocus::Entry { entry, status },
                    status != current_status,
                ));
            }
        }
        order.extend([
            (
                DebugOptionsScreenFocus::DefaultProfile,
                !self
                    .debug_entries
                    .is_using_profile(DebugScreenProfile::Default),
            ),
            (
                DebugOptionsScreenFocus::PerformanceProfile,
                !self
                    .debug_entries
                    .is_using_profile(DebugScreenProfile::Performance),
            ),
            (DebugOptionsScreenFocus::Done, true),
        ]);
        if current == DebugOptionsScreenFocus::List {
            let active = order
                .iter()
                .filter(|(focus, active)| {
                    *active && matches!(focus, DebugOptionsScreenFocus::Entry { .. })
                })
                .map(|(focus, _)| *focus)
                .collect::<Vec<_>>();
            if let Some(focus) = if backwards {
                active.last().copied()
            } else {
                active.first().copied()
            } {
                self.set_debug_options_focus(focus, surface_size);
            }
            return;
        }
        let current_index = order
            .iter()
            .position(|(focus, _)| *focus == current)
            .unwrap_or_default();
        for offset in 1..=order.len() {
            let index = if backwards {
                current_index
                    .checked_sub(offset)
                    .unwrap_or_else(|| order.len() - (offset - current_index))
            } else {
                (current_index + offset) % order.len()
            };
            if order[index].1 {
                self.set_debug_options_focus(order[index].0, surface_size);
                break;
            }
        }
    }

    fn move_debug_options_horizontal_focus(
        &mut self,
        direction: i32,
        surface_size: PhysicalSize<u32>,
    ) {
        let Some(focus) = self.debug_options_screen_focus() else {
            return;
        };
        let default_active = !self
            .debug_entries
            .is_using_profile(DebugScreenProfile::Default);
        let performance_active = !self
            .debug_entries
            .is_using_profile(DebugScreenProfile::Performance);
        let footer_focus = match (focus, direction) {
            (DebugOptionsScreenFocus::DefaultProfile, direction) if direction > 0 => {
                Some(if performance_active {
                    DebugOptionsScreenFocus::PerformanceProfile
                } else {
                    DebugOptionsScreenFocus::Done
                })
            }
            (DebugOptionsScreenFocus::PerformanceProfile, direction) if direction < 0 => {
                default_active.then_some(DebugOptionsScreenFocus::DefaultProfile)
            }
            (DebugOptionsScreenFocus::PerformanceProfile, direction) if direction > 0 => {
                Some(DebugOptionsScreenFocus::Done)
            }
            (DebugOptionsScreenFocus::Done, direction) if direction < 0 => performance_active
                .then_some(DebugOptionsScreenFocus::PerformanceProfile)
                .or_else(|| default_active.then_some(DebugOptionsScreenFocus::DefaultProfile)),
            _ => None,
        };
        if let Some(footer_focus) = footer_focus {
            self.set_debug_options_focus(footer_focus, surface_size);
            return;
        }
        if matches!(
            focus,
            DebugOptionsScreenFocus::DefaultProfile
                | DebugOptionsScreenFocus::PerformanceProfile
                | DebugOptionsScreenFocus::Done
        ) {
            return;
        }
        if focus == DebugOptionsScreenFocus::List {
            let status = if direction < 0 {
                DebugScreenEntryStatus::AlwaysOn
            } else {
                DebugScreenEntryStatus::Never
            };
            let search_text = self
                .screen
                .debug_options()
                .map(|screen| screen.search.text())
                .unwrap_or_default();
            for row in debug_options_screen_rows(&search_text) {
                let DebugOptionsScreenRow::Entry(entry) = row else {
                    continue;
                };
                if self.debug_entries.status(entry) != status {
                    self.set_debug_options_focus(
                        DebugOptionsScreenFocus::Entry { entry, status },
                        surface_size,
                    );
                    break;
                }
            }
            return;
        };
        let DebugOptionsScreenFocus::Entry { entry, status } = focus else {
            return;
        };
        let statuses = [
            DebugScreenEntryStatus::Never,
            DebugScreenEntryStatus::InOverlay,
            DebugScreenEntryStatus::AlwaysOn,
        ];
        let Some(current_index) = statuses.iter().position(|candidate| *candidate == status) else {
            return;
        };
        let mut index = current_index as i32 + direction;
        while let Ok(candidate_index) = usize::try_from(index) {
            let Some(candidate) = statuses.get(candidate_index).copied() else {
                break;
            };
            if self.debug_entries.status(entry) != candidate {
                self.set_debug_options_focus(
                    DebugOptionsScreenFocus::Entry {
                        entry,
                        status: candidate,
                    },
                    surface_size,
                );
                break;
            }
            index += direction;
        }
    }

    fn move_debug_options_vertical_focus(
        &mut self,
        direction: i32,
        surface_size: PhysicalSize<u32>,
    ) {
        let Some(current) = self.debug_options_screen_focus() else {
            return;
        };
        let search_text = self
            .screen
            .debug_options()
            .map(|screen| screen.search.text())
            .unwrap_or_default();
        let entries = debug_options_screen_rows(&search_text)
            .into_iter()
            .filter_map(|row| match row {
                DebugOptionsScreenRow::Entry(entry) => Some(entry),
                DebugOptionsScreenRow::Category { .. } => None,
            })
            .collect::<Vec<_>>();
        match current {
            DebugOptionsScreenFocus::List => {
                let status = DebugScreenEntryStatus::Never;
                let entry = if direction < 0 {
                    entries
                        .iter()
                        .rev()
                        .find(|entry| self.debug_entries.status(**entry) != status)
                } else {
                    entries
                        .iter()
                        .find(|entry| self.debug_entries.status(**entry) != status)
                };
                if let Some(entry) = entry {
                    self.set_debug_options_focus(
                        DebugOptionsScreenFocus::Entry {
                            entry: *entry,
                            status,
                        },
                        surface_size,
                    );
                } else if direction < 0 {
                    self.set_debug_options_focus(DebugOptionsScreenFocus::Search, surface_size);
                } else {
                    let footer_focus = if !self
                        .debug_entries
                        .is_using_profile(DebugScreenProfile::Default)
                    {
                        DebugOptionsScreenFocus::DefaultProfile
                    } else if !self
                        .debug_entries
                        .is_using_profile(DebugScreenProfile::Performance)
                    {
                        DebugOptionsScreenFocus::PerformanceProfile
                    } else {
                        DebugOptionsScreenFocus::Done
                    };
                    self.set_debug_options_focus(footer_focus, surface_size);
                }
            }
            DebugOptionsScreenFocus::Search if direction > 0 => {
                for entry in &entries {
                    let status = DebugScreenEntryStatus::Never;
                    if self.debug_entries.status(*entry) != status {
                        self.set_debug_options_focus(
                            DebugOptionsScreenFocus::Entry {
                                entry: *entry,
                                status,
                            },
                            surface_size,
                        );
                        return;
                    }
                }
                let footer_focus = if self
                    .debug_entries
                    .is_using_profile(DebugScreenProfile::Performance)
                {
                    DebugOptionsScreenFocus::Done
                } else {
                    DebugOptionsScreenFocus::PerformanceProfile
                };
                self.set_debug_options_focus(footer_focus, surface_size);
            }
            DebugOptionsScreenFocus::Entry { entry, status } => {
                let Some(current_index) = entries.iter().position(|candidate| *candidate == entry)
                else {
                    return;
                };
                let mut index = current_index as i32 + direction;
                while let Ok(candidate_index) = usize::try_from(index) {
                    let Some(candidate) = entries.get(candidate_index).copied() else {
                        break;
                    };
                    if self.debug_entries.status(candidate) != status {
                        self.set_debug_options_focus(
                            DebugOptionsScreenFocus::Entry {
                                entry: candidate,
                                status,
                            },
                            surface_size,
                        );
                        return;
                    }
                    index += direction;
                }
                if direction < 0 {
                    self.set_debug_options_focus(DebugOptionsScreenFocus::Search, surface_size);
                } else {
                    let performance_active = !self
                        .debug_entries
                        .is_using_profile(DebugScreenProfile::Performance);
                    let footer_focus = match status {
                        DebugScreenEntryStatus::Never | DebugScreenEntryStatus::InOverlay
                            if performance_active =>
                        {
                            DebugOptionsScreenFocus::PerformanceProfile
                        }
                        DebugScreenEntryStatus::Never
                        | DebugScreenEntryStatus::InOverlay
                        | DebugScreenEntryStatus::AlwaysOn => DebugOptionsScreenFocus::Done,
                    };
                    self.set_debug_options_focus(footer_focus, surface_size);
                }
            }
            DebugOptionsScreenFocus::DefaultProfile
            | DebugOptionsScreenFocus::PerformanceProfile
            | DebugOptionsScreenFocus::Done
                if direction < 0 =>
            {
                let status = DebugScreenEntryStatus::Never;
                let mut moved = false;
                for entry in entries.into_iter().rev() {
                    if self.debug_entries.status(entry) != status {
                        self.set_debug_options_focus(
                            DebugOptionsScreenFocus::Entry { entry, status },
                            surface_size,
                        );
                        moved = true;
                        break;
                    }
                }
                if !moved {
                    self.set_debug_options_focus(DebugOptionsScreenFocus::Search, surface_size);
                }
            }
            _ => {}
        }
    }

    pub(super) fn set_debug_options_focus(
        &mut self,
        focus: DebugOptionsScreenFocus,
        surface_size: PhysicalSize<u32>,
    ) {
        let Some(screen) = self.screen.debug_options_mut() else {
            return;
        };
        screen.focus = focus;
        let DebugOptionsScreenFocus::Entry { entry, .. } = focus else {
            return;
        };
        let rows = debug_options_screen_rows(&screen.search.text());
        let Some(row_index) = rows
            .iter()
            .position(|row| *row == DebugOptionsScreenRow::Entry(entry))
        else {
            return;
        };
        let visible_rows = debug_options_visible_row_count(surface_size);
        if visible_rows == 0 {
            screen.scroll_row = 0;
        } else if row_index < screen.scroll_row {
            screen.scroll_row = row_index;
        } else if row_index >= screen.scroll_row.saturating_add(visible_rows) {
            screen.scroll_row = row_index + 1 - visible_rows;
        }
        screen.scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(rows.len(), visible_rows));
    }

    fn activate_debug_options_focus(&mut self) {
        let Some(focus) = self.debug_options_screen_focus() else {
            return;
        };
        match focus {
            DebugOptionsScreenFocus::Entry { entry, status } => {
                if self.debug_entries.status(entry) != status {
                    self.set_debug_screen_entry_status_inner(entry, status);
                }
            }
            DebugOptionsScreenFocus::DefaultProfile => {
                if !self
                    .debug_entries
                    .is_using_profile(DebugScreenProfile::Default)
                {
                    self.load_debug_screen_profile(DebugScreenProfile::Default);
                }
            }
            DebugOptionsScreenFocus::PerformanceProfile => {
                if !self
                    .debug_entries
                    .is_using_profile(DebugScreenProfile::Performance)
                {
                    self.load_debug_screen_profile(DebugScreenProfile::Performance);
                }
            }
            DebugOptionsScreenFocus::Done => self.close_debug_options_screen(),
            DebugOptionsScreenFocus::Search | DebugOptionsScreenFocus::List => {}
        }
    }

    pub(crate) fn handle_debug_options_screen_text_input(&mut self, text: &str) -> bool {
        let Some(screen) = self.screen.debug_options_mut() else {
            return false;
        };
        if screen.focus == DebugOptionsScreenFocus::Search && screen.search.type_text(text, None) {
            screen.scroll_row = 0;
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        self.handle_debug_options_screen_cursor_moved_with_text_measurer(
            cursor_position,
            surface_size,
            FallbackTextMeasurer.command_suggestion_glyphs(),
        )
    }

    pub(crate) fn handle_debug_options_screen_cursor_moved_with_text_measurer(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        glyphs: &bbb_client_render::HudFontGlyphMap,
    ) -> bool {
        let Some(screen) = self.screen.debug_options_mut() else {
            return false;
        };
        let position = cursor_position.and_then(physical_position_floor_i32);
        screen.cursor_position = position;
        if let (Some(_), Some((_, mouse_y))) = (screen.scrollbar_drag_y, position) {
            drag_debug_options_scrollbar(screen, mouse_y, surface_size);
            return true;
        }
        if let (true, Some((mouse_x, _))) = (screen.search_selecting, position) {
            let (x, _, _, _) = debug_options_search_box_rect(surface_size);
            screen.search.click(
                mouse_x - x - DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET,
                DEBUG_OPTIONS_SEARCH_INNER_WIDTH,
                glyphs,
                true,
            );
        }
        true
    }

    #[cfg(test)]
    pub(crate) fn handle_debug_options_screen_mouse_input(
        &mut self,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
    ) -> bool {
        self.handle_debug_options_screen_mouse_input_with_text_measurer(
            button,
            state,
            cursor_position,
            surface_size,
            reduced_debug_info,
            FallbackTextMeasurer.command_suggestion_glyphs(),
        )
    }

    pub(crate) fn handle_debug_options_screen_mouse_input_with_text_measurer(
        &mut self,
        button: MouseButton,
        state: ElementState,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
        reduced_debug_info: bool,
        glyphs: &bbb_client_render::HudFontGlyphMap,
    ) -> bool {
        if self.screen.debug_options().is_none() {
            return false;
        }
        if let Some(screen) = self.screen.debug_options_mut() {
            screen.cursor_position = cursor_position.and_then(physical_position_floor_i32);
        }
        if matches!((button, state), (MouseButton::Left, ElementState::Released)) {
            if let Some(screen) = self.screen.debug_options_mut() {
                screen.search_selecting = false;
                screen.scrollbar_drag_y = None;
            }
            return true;
        }
        if !matches!((button, state), (MouseButton::Left, ElementState::Pressed)) {
            return true;
        }
        let now = bbb_client_clock::now();
        let double_click = self
            .screen
            .debug_options()
            .is_some_and(|screen| debug_options_screen_is_double_click(screen, now));
        let Some((mouse_x, mouse_y)) = cursor_position.and_then(physical_position_floor_i32) else {
            if let Some(screen) = self.screen.debug_options_mut() {
                screen.search_selecting = false;
                screen.last_left_click_at = None;
            }
            return true;
        };
        if let Some(screen) = self.screen.debug_options_mut() {
            screen.last_left_click_at = Some(now);
        }
        let (x, y, width, height) = debug_options_search_box_rect(surface_size);
        if mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height {
            let shift_down = self.shift_down();
            if let Some(screen) = self.screen.debug_options_mut() {
                screen.focus = DebugOptionsScreenFocus::Search;
                let text_x = mouse_x - x - DEBUG_OPTIONS_SEARCH_TEXT_X_OFFSET;
                if double_click {
                    screen
                        .search
                        .select_word_at(text_x, DEBUG_OPTIONS_SEARCH_INNER_WIDTH, glyphs);
                } else {
                    screen.search.click(
                        text_x,
                        DEBUG_OPTIONS_SEARCH_INNER_WIDTH,
                        glyphs,
                        shift_down,
                    );
                }
                screen.search_selecting = true;
            }
            return true;
        }
        if let Some(screen) = self.screen.debug_options_mut() {
            screen.search_selecting = false;
            screen.scrollbar_drag_y = None;
        }
        let over_scrollbar = self.screen.debug_options().is_some_and(|screen| {
            debug_options_scrollbar_contains(mouse_x, mouse_y, &screen.search.text(), surface_size)
        });
        let over_row = self
            .debug_options_screen_hovered_row(surface_size)
            .is_some();
        if over_scrollbar {
            self.set_debug_options_focus(DebugOptionsScreenFocus::List, surface_size);
        }
        if over_scrollbar {
            if let Some(screen) = self.screen.debug_options_mut() {
                screen.scrollbar_drag_y = Some(mouse_y);
            }
            return true;
        }
        if let Some(profile) = debug_options_profile_button_at(mouse_x, mouse_y, surface_size) {
            if !self.debug_entries.is_using_profile(profile) {
                let focus = match profile {
                    DebugScreenProfile::Default => DebugOptionsScreenFocus::DefaultProfile,
                    DebugScreenProfile::Performance => DebugOptionsScreenFocus::PerformanceProfile,
                };
                self.set_debug_options_focus(focus, surface_size);
                self.load_debug_screen_profile(profile);
                if let Some(screen) = self.screen.debug_options_mut() {
                    screen.scroll_row = 0;
                }
            }
            return true;
        }
        if debug_options_done_button_contains(mouse_x, mouse_y, surface_size) {
            self.close_debug_options_screen();
            return true;
        }
        if let Some((entry, status)) =
            self.debug_options_status_button_at(mouse_x, mouse_y, surface_size, reduced_debug_info)
        {
            if self.debug_entries.status(entry) != status {
                self.set_debug_options_focus(
                    DebugOptionsScreenFocus::Entry { entry, status },
                    surface_size,
                );
                self.set_debug_screen_entry_status_inner(entry, status);
            } else if !matches!(
                self.debug_options_screen_focus(),
                Some(DebugOptionsScreenFocus::Entry {
                    entry: focused_entry,
                    ..
                }) if focused_entry == entry
            ) {
                self.set_debug_options_focus(DebugOptionsScreenFocus::List, surface_size);
            }
            return true;
        }
        if over_row
            && !matches!(
                self.debug_options_screen_focus(),
                Some(DebugOptionsScreenFocus::Entry { .. })
            )
        {
            self.set_debug_options_focus(DebugOptionsScreenFocus::List, surface_size);
        }
        true
    }

    pub(crate) fn handle_debug_options_screen_mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(screen) = self.screen.debug_options_mut() else {
            return false;
        };
        let rows = debug_options_screen_rows(&screen.search.text());
        let visible_rows = debug_options_visible_row_count(surface_size);
        let max_scroll = debug_options_max_scroll_row(rows.len(), visible_rows);
        let delta_rows = debug_options_scroll_delta_rows(delta);
        if delta_rows < 0 {
            screen.scroll_row = screen
                .scroll_row
                .saturating_add(delta_rows.unsigned_abs() as usize);
        } else if delta_rows > 0 {
            screen.scroll_row = screen.scroll_row.saturating_sub(delta_rows as usize);
        }
        screen.scroll_row = screen.scroll_row.min(max_scroll);
        true
    }

    pub(crate) fn debug_game_mode_switcher_selected(&self) -> Option<GameType> {
        self.screen
            .game_mode_switcher()
            .map(|switcher| switcher.selected)
    }

    pub(crate) fn handle_debug_game_mode_switcher_cursor_moved(
        &mut self,
        cursor_position: Option<PhysicalPosition<f64>>,
        surface_size: PhysicalSize<u32>,
    ) -> bool {
        let Some(switcher) = self.screen.game_mode_switcher_mut() else {
            return false;
        };
        let Some((mouse_x, mouse_y)) = cursor_position.and_then(debug_game_mode_switcher_mouse_pos)
        else {
            return true;
        };
        let mouse_pos = (mouse_x, mouse_y);
        let Some(first_mouse_position) = switcher.first_mouse_position else {
            switcher.first_mouse_position = Some(mouse_pos);
            return true;
        };
        if first_mouse_position == mouse_pos {
            return true;
        }
        if let Some(hovered) = debug_game_mode_switcher_hovered_game_type(mouse_pos, surface_size) {
            switcher.selected = hovered;
        }
        true
    }

    fn debug_options_status_button_at(
        &self,
        mouse_x: i32,
        mouse_y: i32,
        surface_size: PhysicalSize<u32>,
        _reduced_debug_info: bool,
    ) -> Option<(DebugScreenEntryId, DebugScreenEntryStatus)> {
        let screen = self.screen.debug_options()?;
        let row_index = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
        let rows = debug_options_screen_rows(&screen.search.text());
        let visible_rows = debug_options_visible_row_count(surface_size);
        let scroll_row = screen
            .scroll_row
            .min(debug_options_max_scroll_row(rows.len(), visible_rows));
        let DebugOptionsScreenRow::Entry(entry) = *rows.get(scroll_row + row_index)? else {
            return None;
        };
        let buttons_start_x = debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
            - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
        if mouse_x < buttons_start_x {
            return None;
        }
        let button_index =
            ((mouse_x - buttons_start_x) / DEBUG_OPTIONS_STATUS_BUTTON_WIDTH).clamp(0, 2);
        let status = match button_index {
            0 => DebugScreenEntryStatus::Never,
            1 => DebugScreenEntryStatus::InOverlay,
            _ => DebugScreenEntryStatus::AlwaysOn,
        };
        Some((entry, status))
    }
}

fn debug_options_screen_rows(search: &str) -> Vec<DebugOptionsScreenRow> {
    let mut rows = Vec::new();
    let mut current_category = None;
    for entry in DebugScreenEntryId::all_debug_options_ordered()
        .filter(|entry| entry.path().contains(search))
    {
        let category = entry.category_label();
        if current_category != Some(category) {
            rows.push(DebugOptionsScreenRow::Category { label: category });
            current_category = Some(category);
        }
        rows.push(DebugOptionsScreenRow::Entry(entry));
    }
    rows
}

fn debug_options_visible_row_count(surface_size: PhysicalSize<u32>) -> usize {
    (debug_options_list_height(surface_size) / DEBUG_OPTIONS_ROW_HEIGHT).max(0) as usize
}

fn debug_options_max_scroll_row(total_rows: usize, visible_rows: usize) -> usize {
    total_rows.saturating_sub(visible_rows)
}

pub(super) fn debug_options_list_height(surface_size: PhysicalSize<u32>) -> i32 {
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (height - DEBUG_OPTIONS_HEADER_HEIGHT - DEBUG_OPTIONS_FOOTER_HEIGHT).max(0)
}

pub(super) fn debug_options_search_box_rect(
    surface_size: PhysicalSize<u32>,
) -> (i32, i32, i32, i32) {
    let ((x, y, width, height), _) = hud_debug_options_search_box_rects(surface_size);
    (x, y, width as i32, height as i32)
}

pub(super) fn debug_options_scrollbar_x(surface_size: PhysicalSize<u32>) -> i32 {
    debug_options_content_x(surface_size)
        + DEBUG_OPTIONS_ROW_WIDTH
        + DEBUG_OPTIONS_SCROLLBAR_WIDTH
        + 2
}

fn debug_options_scrollbar_contains(
    mouse_x: i32,
    mouse_y: i32,
    search_text: &str,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let total_rows = debug_options_screen_rows(search_text).len();
    let list_height = debug_options_list_height(surface_size);
    if !debug_options_scrollbar_scrollable(total_rows, list_height) {
        return false;
    }
    let scrollbar_x = debug_options_scrollbar_x(surface_size);
    mouse_x >= scrollbar_x
        && mouse_x <= scrollbar_x + DEBUG_OPTIONS_SCROLLBAR_WIDTH
        && mouse_y >= DEBUG_OPTIONS_HEADER_HEIGHT
        && mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT + list_height
}

fn drag_debug_options_scrollbar(
    screen: &mut DebugOptionsScreenState,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) {
    let Some(last_y) = screen.scrollbar_drag_y else {
        return;
    };
    let total_rows = debug_options_screen_rows(&screen.search.text()).len();
    screen.scroll_row = debug_options_scrollbar_drag_scroll_row(
        screen.scroll_row,
        total_rows,
        surface_size,
        last_y,
        mouse_y,
    );
    screen.scrollbar_drag_y = Some(mouse_y);
}

pub(super) fn debug_options_scrollbar_drag_scroll_row(
    scroll_row: usize,
    total_rows: usize,
    surface_size: PhysicalSize<u32>,
    last_y: i32,
    mouse_y: i32,
) -> usize {
    let visible_rows = debug_options_visible_row_count(surface_size);
    let max_scroll_row = debug_options_max_scroll_row(total_rows, visible_rows);
    if max_scroll_row == 0 {
        return 0;
    }
    let list_height = debug_options_list_height(surface_size);
    if mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT {
        return 0;
    }
    if mouse_y > DEBUG_OPTIONS_HEADER_HEIGHT + list_height {
        return max_scroll_row;
    }
    let content_height = debug_options_scrollbar_content_height(total_rows);
    let max_scroll_amount = (content_height - list_height).max(0);
    if max_scroll_amount == 0 {
        return 0;
    }
    let scroller_height = debug_options_scroller_height(list_height, content_height);
    let y_drag_scale =
        f64::from(max_scroll_amount.max(1)) / f64::from((list_height - scroller_height).max(1));
    let y_drag_scale = y_drag_scale.max(1.0);
    let scroll_amount = i32::try_from(scroll_row)
        .unwrap_or(i32::MAX / DEBUG_OPTIONS_ROW_HEIGHT)
        .saturating_mul(DEBUG_OPTIONS_ROW_HEIGHT)
        .min(max_scroll_amount);
    let dragged = f64::from(scroll_amount) + f64::from(mouse_y - last_y) * y_drag_scale;
    ((dragged.clamp(0.0, f64::from(max_scroll_amount)) / f64::from(DEBUG_OPTIONS_ROW_HEIGHT))
        .round() as usize)
        .min(max_scroll_row)
}

fn debug_options_scrollbar_scrollable(total_rows: usize, list_height: i32) -> bool {
    list_height > 8 && debug_options_scrollbar_content_height(total_rows) > list_height
}

fn debug_options_scrollbar_content_height(total_rows: usize) -> i32 {
    i32::try_from(total_rows)
        .unwrap_or(i32::MAX / DEBUG_OPTIONS_ROW_HEIGHT)
        .saturating_mul(DEBUG_OPTIONS_ROW_HEIGHT)
}

fn debug_options_scroller_height(list_height: i32, content_height: i32) -> i32 {
    let max_scroller_height = (list_height - 8).max(1);
    let min_scroller_height = DEBUG_OPTIONS_SCROLLBAR_MIN_HEIGHT.min(max_scroller_height);
    list_height
        .saturating_mul(list_height)
        .checked_div(content_height.max(1))
        .unwrap_or(max_scroller_height)
        .clamp(min_scroller_height, max_scroller_height)
}

fn debug_options_screen_is_double_click(screen: &DebugOptionsScreenState, now: Instant) -> bool {
    screen
        .last_left_click_at
        .and_then(|last| now.checked_duration_since(last))
        .is_some_and(|elapsed| elapsed < DEBUG_OPTIONS_DOUBLE_CLICK_THRESHOLD)
}

fn debug_options_row_index_at(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> Option<usize> {
    let x = debug_options_content_x(surface_size);
    if mouse_x < x || mouse_x >= x + DEBUG_OPTIONS_ROW_WIDTH {
        return None;
    }
    if mouse_y < DEBUG_OPTIONS_HEADER_HEIGHT {
        return None;
    }
    let row_index = (mouse_y - DEBUG_OPTIONS_HEADER_HEIGHT) / DEBUG_OPTIONS_ROW_HEIGHT;
    (row_index >= 0 && (row_index as usize) < debug_options_visible_row_count(surface_size))
        .then_some(row_index as usize)
}

fn debug_options_not_allowed_tooltip_at(
    search_text: &str,
    scroll_row: usize,
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
    reduced_debug_info: bool,
) -> Option<(i32, i32)> {
    let row_index = debug_options_row_index_at(mouse_x, mouse_y, surface_size)?;
    let rows = debug_options_screen_rows(search_text);
    let visible_rows = debug_options_visible_row_count(surface_size);
    let scroll_row = scroll_row.min(debug_options_max_scroll_row(rows.len(), visible_rows));
    let DebugOptionsScreenRow::Entry(entry) = *rows.get(scroll_row + row_index)? else {
        return None;
    };
    if entry.is_allowed(reduced_debug_info) {
        return None;
    }
    let buttons_start_x = debug_options_content_x(surface_size) + DEBUG_OPTIONS_ROW_WIDTH
        - DEBUG_OPTIONS_STATUS_BUTTON_WIDTH * 3;
    (mouse_x < buttons_start_x).then_some((mouse_x, mouse_y))
}

fn debug_options_profile_button_at(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> Option<DebugScreenProfile> {
    let y = debug_options_footer_button_y(surface_size);
    if mouse_y < y || mouse_y >= y + DEBUG_OPTIONS_ROW_HEIGHT {
        return None;
    }
    let (default_x, performance_x, _) = debug_options_footer_button_xs(surface_size);
    if mouse_x >= default_x && mouse_x < default_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH {
        return Some(DebugScreenProfile::Default);
    }
    if mouse_x >= performance_x && mouse_x < performance_x + DEBUG_OPTIONS_PROFILE_BUTTON_WIDTH {
        return Some(DebugScreenProfile::Performance);
    }
    None
}

fn debug_options_done_button_contains(
    mouse_x: i32,
    mouse_y: i32,
    surface_size: PhysicalSize<u32>,
) -> bool {
    let y = debug_options_footer_button_y(surface_size);
    let (_, _, done_x) = debug_options_footer_button_xs(surface_size);
    mouse_x >= done_x
        && mouse_x < done_x + DEBUG_OPTIONS_DONE_BUTTON_WIDTH
        && mouse_y >= y
        && mouse_y < y + DEBUG_OPTIONS_ROW_HEIGHT
}

fn debug_options_scroll_delta_rows(delta: MouseScrollDelta) -> i32 {
    match delta {
        MouseScrollDelta::LineDelta(_, y) => y.round() as i32,
        MouseScrollDelta::PixelDelta(position) => {
            (position.y / f64::from(DEBUG_OPTIONS_ROW_HEIGHT))
                .round()
                .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
        }
    }
}
