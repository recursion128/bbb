use bbb_item_model::NativeItemRuntime;
use bbb_renderer::{
    HudDebugOptionsEntryStatus, HudDebugOptionsRow, HudDebugOptionsScreen, HudDebugOptionsTooltip,
};
use bbb_world::WorldStore;
use winit::dpi::PhysicalSize;

use crate::{
    debug_entries::{DebugScreenEntryId, DebugScreenEntryStatus},
    input::{
        ClientInputState, DebugOptionsScreenFocus, DebugOptionsScreenHudRow, DebugOptionsScreenRow,
    },
    narrator_runtime::{NarratedElementKind, ScreenNarrationFrame},
};

use super::{localized_gui_text, localized_gui_text_with_args, wall_clock_millis};

pub(super) fn hud_debug_options_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: PhysicalSize<u32>,
) -> Option<HudDebugOptionsScreen> {
    let state =
        input.debug_options_screen_hud_state(surface_size, input.show_only_reduced_info(world))?;
    let rows = state
        .rows
        .into_iter()
        .map(|row| match row {
            DebugOptionsScreenHudRow::Category { label } => {
                let key = match label.as_str() {
                    "Debug Renderers" => "debug.options.category.renderer",
                    _ => "debug.options.category.text",
                };
                HudDebugOptionsRow::Category {
                    label: localized_gui_text(item_runtime, key, &label),
                }
            }
            DebugOptionsScreenHudRow::Entry {
                path,
                status,
                hovered_status,
                focused_status,
                allowed,
                ..
            } => HudDebugOptionsRow::Entry {
                path,
                status: hud_debug_options_status(status),
                hovered_status: hovered_status.map(hud_debug_options_status),
                focused_status: focused_status.map(hud_debug_options_status),
                allowed,
            },
        })
        .collect();
    Some(HudDebugOptionsScreen {
        title: localized_gui_text(item_runtime, "debug.options.title", "Debug Options"),
        warning: localized_gui_text(
            item_runtime,
            "debug.options.warning",
            "These options are for testing purposes only. They may slow down your computer, crash the game, or eat your pet rock.",
        ),
        search_text: state.search_text,
        search_cursor: state.search_cursor,
        search_selection: state.search_selection,
        search_cursor_visible: (wall_clock_millis() / 300) % 2 == 0,
        search_focused: state.search_focused,
        search_hint: localized_gui_text(item_runtime, "debug.options.search", "Search..."),
        never_label: localized_gui_text(item_runtime, "options.off", "OFF"),
        overlay_label: localized_gui_text(
            item_runtime,
            "debug.entry.overlay",
            "In Overlay",
        ),
        always_label: localized_gui_text(item_runtime, "debug.entry.always", "Always"),
        rows,
        tooltip: state.tooltip.map(|tooltip| HudDebugOptionsTooltip {
            text: localized_gui_text(
                item_runtime,
                "debug.options.notAllowed.tooltip",
                &tooltip.text,
            ),
            x: tooltip.x,
            y: tooltip.y,
        }),
        scroll_row: state.scroll_row,
        total_rows: state.total_rows,
        visible_rows: state.visible_rows,
        default_profile_active: state.default_profile_active,
        default_profile_hovered: state.default_profile_hovered,
        default_profile_focused: state.default_profile_focused,
        default_profile_label: localized_gui_text(
            item_runtime,
            "debug.options.profile.default",
            "Default profile",
        ),
        performance_profile_active: state.performance_profile_active,
        performance_profile_hovered: state.performance_profile_hovered,
        performance_profile_focused: state.performance_profile_focused,
        performance_profile_label: localized_gui_text(
            item_runtime,
            "debug.options.profile.performance",
            "Performance profile",
        ),
        done_hovered: state.done_hovered,
        done_focused: state.done_focused,
        done_label: localized_gui_text(item_runtime, "gui.done", "Done"),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebugOptionsNarrationWidget {
    Search,
    List,
    DefaultProfile,
    PerformanceProfile,
    Done,
}

pub(super) fn debug_options_screen_narration_frame(
    input: &ClientInputState,
    screen: &HudDebugOptionsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: PhysicalSize<u32>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());
    let focus = input
        .debug_options_screen_focus()
        .unwrap_or(DebugOptionsScreenFocus::Search);
    let focused_widget = match focus {
        DebugOptionsScreenFocus::Search => DebugOptionsNarrationWidget::Search,
        DebugOptionsScreenFocus::List | DebugOptionsScreenFocus::Entry { .. } => {
            DebugOptionsNarrationWidget::List
        }
        DebugOptionsScreenFocus::DefaultProfile => DebugOptionsNarrationWidget::DefaultProfile,
        DebugOptionsScreenFocus::PerformanceProfile => {
            DebugOptionsNarrationWidget::PerformanceProfile
        }
        DebugOptionsScreenFocus::Done => DebugOptionsNarrationWidget::Done,
    };
    let mut active_widgets = vec![
        DebugOptionsNarrationWidget::Search,
        DebugOptionsNarrationWidget::List,
    ];
    if screen.default_profile_active {
        active_widgets.push(DebugOptionsNarrationWidget::DefaultProfile);
    }
    if screen.performance_profile_active {
        active_widgets.push(DebugOptionsNarrationWidget::PerformanceProfile);
    }
    active_widgets.push(DebugOptionsNarrationWidget::Done);
    let focused_widget_active = active_widgets.contains(&focused_widget);
    let widget = focused_widget_active.then_some(focused_widget).or_else(|| {
        if input.debug_options_screen_search_hovered(surface_size) {
            Some(DebugOptionsNarrationWidget::Search)
        } else if input
            .debug_options_screen_hovered_row(surface_size)
            .is_some()
        {
            Some(DebugOptionsNarrationWidget::List)
        } else if screen.default_profile_active && screen.default_profile_hovered {
            Some(DebugOptionsNarrationWidget::DefaultProfile)
        } else if screen.performance_profile_active && screen.performance_profile_hovered {
            Some(DebugOptionsNarrationWidget::PerformanceProfile)
        } else if screen.done_hovered {
            Some(DebugOptionsNarrationWidget::Done)
        } else {
            None
        }
    });
    let Some(widget) = widget else {
        frame.add(
            NarratedElementKind::Usage,
            0,
            localized_gui_text(
                item_runtime,
                "narrator.screen.usage",
                "Use mouse cursor or Tab button to select element",
            ),
        );
        return frame;
    };
    let Some(position) = active_widgets
        .iter()
        .position(|candidate| *candidate == widget)
    else {
        frame.add(
            NarratedElementKind::Usage,
            0,
            localized_gui_text(
                item_runtime,
                "narrator.screen.usage",
                "Use mouse cursor or Tab button to select element",
            ),
        );
        return frame;
    };
    frame.add(
        NarratedElementKind::Position,
        0,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.screen",
            "Screen element %1$s out of %2$s",
            &[
                &(position + 1).to_string(),
                &active_widgets.len().to_string(),
            ],
        ),
    );
    if focused_widget_active {
        frame.add(
            NarratedElementKind::Usage,
            0,
            localized_gui_text(
                item_runtime,
                "narration.component_list.usage",
                "Press Tab to navigate to the next element",
            ),
        );
    }
    match widget {
        DebugOptionsNarrationWidget::Search => frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "gui.narrate.editBox",
                "%1$s edit box: %2$s",
                &[&screen.search_hint, &screen.search_text],
            ),
        ),
        DebugOptionsNarrationWidget::List => debug_options_list_narration(
            &mut frame,
            input,
            screen,
            item_runtime,
            surface_size,
            focus,
        ),
        DebugOptionsNarrationWidget::DefaultProfile => debug_options_button_narration(
            &mut frame,
            item_runtime,
            &screen.default_profile_label,
            focused_widget_active,
        ),
        DebugOptionsNarrationWidget::PerformanceProfile => debug_options_button_narration(
            &mut frame,
            item_runtime,
            &screen.performance_profile_label,
            focused_widget_active,
        ),
        DebugOptionsNarrationWidget::Done => debug_options_button_narration(
            &mut frame,
            item_runtime,
            &screen.done_label,
            focused_widget_active,
        ),
    }
    frame
}

fn debug_options_list_narration(
    frame: &mut ScreenNarrationFrame,
    input: &ClientInputState,
    screen: &HudDebugOptionsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: PhysicalSize<u32>,
    focus: DebugOptionsScreenFocus,
) {
    let hovered = input.debug_options_screen_hovered_row(surface_size);
    let selected = hovered.or_else(|| {
        let DebugOptionsScreenFocus::Entry { entry, .. } = focus else {
            return None;
        };
        screen
            .rows
            .iter()
            .position(|row| {
                matches!(
                    row,
                    HudDebugOptionsRow::Entry { path, .. } if path == entry.path()
                )
            })
            .map(|visible| {
                (
                    screen.scroll_row + visible,
                    DebugOptionsScreenRow::Entry(entry),
                )
            })
    });
    let Some((row_index, row)) = selected else {
        return;
    };
    if screen.total_rows > 1 {
        frame.add(
            NarratedElementKind::Position,
            1,
            localized_gui_text_with_args(
                item_runtime,
                "narrator.position.list",
                "Selected list row %1$s out of %2$s",
                &[&(row_index + 1).to_string(), &screen.total_rows.to_string()],
            ),
        );
    }
    match row {
        DebugOptionsScreenRow::Category { label } => frame.add(
            NarratedElementKind::Title,
            1,
            localized_gui_text(
                item_runtime,
                if label == "Debug Renderers" {
                    "debug.options.category.renderer"
                } else {
                    "debug.options.category.text"
                },
                label,
            ),
        ),
        DebugOptionsScreenRow::Entry(entry) => {
            let hovered_status = row_index
                .checked_sub(screen.scroll_row)
                .and_then(|visible| match screen.rows.get(visible) {
                    Some(HudDebugOptionsRow::Entry { hovered_status, .. }) => *hovered_status,
                    _ => None,
                });
            let focused_status = match focus {
                DebugOptionsScreenFocus::Entry {
                    entry: focused_entry,
                    status,
                } if focused_entry == entry => Some(status),
                _ => None,
            };
            if let Some(status) =
                focused_status.or_else(|| hovered_status.map(debug_options_status_from_hud))
            {
                debug_options_status_button_narration(
                    frame,
                    input,
                    item_runtime,
                    entry,
                    status,
                    focused_status == Some(status),
                );
            }
        }
    }
}

fn debug_options_status_button_narration(
    frame: &mut ScreenNarrationFrame,
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    entry: DebugScreenEntryId,
    status: DebugScreenEntryStatus,
    focused: bool,
) {
    let current_status = input.debug_screen_entry_status(entry);
    let (current_key, current_english) = match current_status {
        DebugScreenEntryStatus::AlwaysOn => (
            "debug.entry.currently.alwaysOn",
            "%1$s: Currently always on",
        ),
        DebugScreenEntryStatus::InOverlay => (
            "debug.entry.currently.inOverlay",
            "%1$s: Currently only in debug overlay",
        ),
        DebugScreenEntryStatus::Never => ("debug.entry.currently.never", "%1$s: Currently off"),
    };
    let current =
        localized_gui_text_with_args(item_runtime, current_key, current_english, &[entry.path()]);
    let value = match status {
        DebugScreenEntryStatus::AlwaysOn => {
            localized_gui_text(item_runtime, "debug.entry.always", "Always")
        }
        DebugScreenEntryStatus::InOverlay => {
            localized_gui_text(item_runtime, "debug.entry.overlay", "In Overlay")
        }
        DebugScreenEntryStatus::Never => localized_gui_text(item_runtime, "options.off", "OFF"),
    };
    frame.add(
        NarratedElementKind::Position,
        2,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.object_list",
            "Selected row element %1$s out of %2$s",
            &[
                &(match status {
                    DebugScreenEntryStatus::Never => 1,
                    DebugScreenEntryStatus::InOverlay => 2,
                    DebugScreenEntryStatus::AlwaysOn => 3,
                })
                .to_string(),
                "3",
            ],
        ),
    );
    frame.add(
        NarratedElementKind::Title,
        2,
        localized_gui_text_with_args(
            item_runtime,
            "options.generic_value",
            "%1$s: %2$s",
            &[&current, &value],
        ),
    );
    if current_status != status {
        frame.add(
            NarratedElementKind::Usage,
            2,
            localized_gui_text(
                item_runtime,
                if focused {
                    "narration.button.usage.focused"
                } else {
                    "narration.button.usage.hovered"
                },
                if focused {
                    "Press Enter to activate"
                } else {
                    "Left click to activate"
                },
            ),
        );
    }
}

fn debug_options_status_from_hud(status: HudDebugOptionsEntryStatus) -> DebugScreenEntryStatus {
    match status {
        HudDebugOptionsEntryStatus::AlwaysOn => DebugScreenEntryStatus::AlwaysOn,
        HudDebugOptionsEntryStatus::InOverlay => DebugScreenEntryStatus::InOverlay,
        HudDebugOptionsEntryStatus::Never => DebugScreenEntryStatus::Never,
    }
}

fn debug_options_button_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    label: &str,
    focused: bool,
) {
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%1$s button", &[label]),
    );
    frame.add(
        NarratedElementKind::Usage,
        1,
        localized_gui_text(
            item_runtime,
            if focused {
                "narration.button.usage.focused"
            } else {
                "narration.button.usage.hovered"
            },
            if focused {
                "Press Enter to activate"
            } else {
                "Left click to activate"
            },
        ),
    );
}

fn hud_debug_options_status(status: DebugScreenEntryStatus) -> HudDebugOptionsEntryStatus {
    match status {
        DebugScreenEntryStatus::AlwaysOn => HudDebugOptionsEntryStatus::AlwaysOn,
        DebugScreenEntryStatus::InOverlay => HudDebugOptionsEntryStatus::InOverlay,
        DebugScreenEntryStatus::Never => HudDebugOptionsEntryStatus::Never,
    }
}
