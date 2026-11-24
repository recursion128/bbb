use super::*;

pub(super) fn sync_stats_screen_interaction(
    stats: &mut StatsScreen,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) {
    let data = StatsScreenData::new(world, item_runtime);
    let row_counts = data.row_counts();
    stats.sync_tab_row_counts(row_counts);
    let layout = StatsScreenLayout::new(stats, row_counts, surface_size);
    if layout.active_tab != stats.tab {
        stats.reset_scroll();
    }
    stats.sync_interaction(stats.loading, layout.active_tab, layout.total_rows);
    stats.sync_scroll_metrics(layout.max_scroll_amount, layout.scroller_height);
    stats.keep_focus_visible(layout.viewport_height, layout.row_height);
}

pub(super) fn hud_stats_screen(
    stats: &StatsScreen,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    terrain_textures: &TerrainTextureState,
    keybind_context: ItemModelKeybindContext,
    partial_tick: f32,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> HudStatsScreen {
    let data = StatsScreenData::new(world, item_runtime);
    let row_counts = data.row_counts();
    let layout = StatsScreenLayout::new(stats, row_counts, surface_size);
    let StatsScreenLayout {
        active_tab,
        total_rows,
        row_height,
        max_scroll_amount,
        scroller_height,
        scroll_amount,
        scroll_row,
        visible_rows,
        ..
    } = layout;
    let is_loading = stats.loading;
    let items_tab_enabled = row_counts[1] > 0;
    let mobs_tab_enabled = row_counts[2] > 0;
    let rows = if is_loading {
        Vec::new()
    } else {
        match active_tab {
            0 => stats_general_rows(&data, item_runtime, scroll_row, visible_rows),
            2 => stats_mob_rows(&data, item_runtime, scroll_row, visible_rows),
            _ => Vec::new(),
        }
    };
    let item_candidates = if !is_loading && active_tab == 1 {
        stats_item_rows(stats.item_effective_sort, &data, item_runtime)
    } else {
        Vec::new()
    };
    let item_rows = if item_candidates.is_empty() || visible_rows == 0 {
        Vec::new()
    } else {
        let item_models = HudItemModelContext::new(world, item_runtime);
        item_candidates
            .iter()
            .skip(scroll_row)
            .take(visible_rows)
            .map(|candidate| {
                let stack = ItemStackSummary {
                    item_id: Some(candidate.item_id),
                    count: 1,
                    component_patch: Default::default(),
                };
                let projection = item_models.project(
                    &stack,
                    HudItemSlotState {
                        keybind: keybind_context,
                        ..HudItemSlotState::default()
                    },
                    world.local_player_team_color_rgb(),
                );
                HudStatsItemRow {
                    item_id: candidate.item_id,
                    resource_id: candidate.resource_id.to_owned(),
                    label: stats_item_label(item_runtime, candidate.resource_id),
                    values: candidate
                        .raw_values
                        .map(|value| value.map(format_grouped_integer)),
                    icon: projection
                        .as_ref()
                        .and_then(|projection| projection.icon(partial_tick)),
                    block_model: projection
                        .as_ref()
                        .and_then(|projection| projection.block_model(terrain_textures)),
                }
            })
            .collect()
    };
    let (item_sort_column, item_sort_order) = stats.item_sort();
    let item_column_labels = [
        localized_gui_text(item_runtime, "stat_type.minecraft.mined", "Mined"),
        localized_gui_text(item_runtime, "stat_type.minecraft.broken", "Broken"),
        localized_gui_text(item_runtime, "stat_type.minecraft.crafted", "Crafted"),
        localized_gui_text(item_runtime, "stat_type.minecraft.used", "Used"),
        localized_gui_text(item_runtime, "stat_type.minecraft.picked_up", "Picked Up"),
        localized_gui_text(item_runtime, "stat_type.minecraft.dropped", "Dropped"),
    ];
    let item_hovered_column = stats.item_hovered_column(surface_size, scroll_amount);
    let item_tooltip_text = if active_tab == 1 {
        stats
            .hovered_item_row(surface_size, scroll_amount)
            .and_then(|row| item_candidates.get(row))
            .map(|row| stats_item_label(item_runtime, row.resource_id))
            .or_else(|| item_hovered_column.map(|column| item_column_labels[column].clone()))
    } else {
        None
    };
    let tooltip_text = item_tooltip_text.or_else(|| {
        (!is_loading)
            .then(|| match stats.hovered_tab(surface_size) {
                Some(1) if !items_tab_enabled => Some(localized_gui_text(
                    item_runtime,
                    "gui.stats.none_found",
                    "No statistics found",
                )),
                Some(2) if !mobs_tab_enabled => Some(localized_gui_text(
                    item_runtime,
                    "gui.stats.none_found",
                    "No statistics found",
                )),
                _ => None,
            })
            .flatten()
    });
    let tooltip = tooltip_text.and_then(|text| {
        stats
            .cursor_position
            .map(|(x, y)| HudDebugOptionsTooltip { text, x, y })
    });
    HudStatsScreen {
        title: localized_gui_text(item_runtime, "gui.stats", "Stats"),
        loading_text: localized_gui_text(
            item_runtime,
            "multiplayer.downloadingStats",
            "Downloading statistics...",
        ),
        is_loading,
        general_tab: localized_gui_text(item_runtime, "stat.generalButton", "General"),
        items_tab: localized_gui_text(item_runtime, "stat.itemsButton", "Items"),
        mobs_tab: localized_gui_text(item_runtime, "stat.mobsButton", "Mobs"),
        active_tab,
        items_tab_enabled: items_tab_enabled && !is_loading,
        mobs_tab_enabled: mobs_tab_enabled && !is_loading,
        rows,
        item_rows,
        item_column_labels,
        item_no_value: localized_gui_text(item_runtime, "stats.none", "-"),
        item_sort_column,
        item_sort_order,
        item_hovered_column,
        item_focused_column: match stats.focus {
            StatsScreenFocus::ItemHeader(column) => Some(column),
            _ => None,
        },
        scroll_row,
        scroll_amount,
        max_scroll_amount,
        scroller_height,
        total_rows,
        visible_rows,
        row_height,
        tooltip,
        done: localized_gui_text(item_runtime, "gui.done", "Done"),
        done_hovered: stats_screen_done_button_contains(stats.cursor_position, surface_size),
        done_focused: stats.focus == StatsScreenFocus::Done,
    }
}

struct StatsScreenLayout {
    active_tab: usize,
    total_rows: usize,
    row_height: i32,
    viewport_height: i32,
    max_scroll_amount: i32,
    scroller_height: i32,
    scroll_amount: i32,
    scroll_row: usize,
    visible_rows: usize,
}

impl StatsScreenLayout {
    fn new(
        stats: &StatsScreen,
        row_counts: [usize; 3],
        surface_size: winit::dpi::PhysicalSize<u32>,
    ) -> Self {
        let active_tab = match stats.tab {
            1 | 2 if !stats.loading && row_counts[stats.tab] > 0 => stats.tab,
            _ => 0,
        };
        let row_height = match active_tab {
            1 => 22,
            2 => 36,
            _ => 14,
        };
        let total_rows = row_counts[active_tab];
        let viewport_height = stats_screen_viewport_height(surface_size, row_height);
        let content_entries = if stats.loading {
            0
        } else {
            total_rows.saturating_add(usize::from(active_tab == 1))
        };
        let content_height = if content_entries == 0 {
            0
        } else {
            i32::try_from(content_entries)
                .unwrap_or(i32::MAX / row_height)
                .saturating_mul(row_height)
                .saturating_add(4)
        };
        let max_scroll_amount = content_height.saturating_sub(viewport_height).max(0);
        let scroller_height =
            stats_screen_scroller_height(viewport_height, content_height, max_scroll_amount);
        let scroll_amount = if active_tab != stats.tab {
            0.0
        } else {
            stats.scroll_amount
        };
        let scroll_amount = stats.focused_scroll_amount(
            viewport_height,
            row_height,
            max_scroll_amount,
            scroll_amount,
        ) as i32;
        let scroll_row =
            stats_screen_first_data_row(active_tab, scroll_amount, row_height).min(total_rows);
        let first_row_offset =
            stats_screen_first_data_row_offset(active_tab, scroll_row, scroll_amount, row_height);
        let visible_rows = if stats.loading {
            0
        } else {
            stats_screen_projected_row_count(
                total_rows.saturating_sub(scroll_row),
                viewport_height,
                first_row_offset,
                row_height,
            )
        };
        Self {
            active_tab,
            total_rows,
            row_height,
            viewport_height,
            max_scroll_amount,
            scroller_height,
            scroll_amount,
            scroll_row,
            visible_rows,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatsNarrationWidget {
    Tabs,
    List,
    Done,
}

pub(super) fn stats_screen_narration_frame(
    stats: &StatsScreen,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    surface_size: winit::dpi::PhysicalSize<u32>,
) -> ScreenNarrationFrame {
    let mut frame = ScreenNarrationFrame::default();
    frame.add(NarratedElementKind::Title, 0, screen.title.clone());

    let focus = stats.focus;
    let focused_widget = match focus {
        StatsScreenFocus::Tabs => Some(StatsNarrationWidget::Tabs),
        StatsScreenFocus::List
        | StatsScreenFocus::GeneralRow(_)
        | StatsScreenFocus::ItemHeader(_)
        | StatsScreenFocus::ItemRow(_)
        | StatsScreenFocus::MobRow(_) => Some(StatsNarrationWidget::List),
        StatsScreenFocus::Done => Some(StatsNarrationWidget::Done),
        StatsScreenFocus::None => None,
    };
    let hovered_tab = stats.hovered_tab(surface_size);
    let hovered_row = stats.hovered_row(surface_size);
    let list_hovered = hovered_row.is_some() || screen.item_hovered_column.is_some();
    let selected_widget = focused_widget.or_else(|| {
        hovered_tab
            .map(|_| StatsNarrationWidget::Tabs)
            .or_else(|| list_hovered.then_some(StatsNarrationWidget::List))
            .or_else(|| screen.done_hovered.then_some(StatsNarrationWidget::Done))
    });
    let active_widget_count = if screen.is_loading { 2 } else { 3 };
    if let Some(widget) = selected_widget {
        let position = match widget {
            StatsNarrationWidget::Tabs => 1,
            StatsNarrationWidget::List => 2,
            StatsNarrationWidget::Done if screen.is_loading => 2,
            StatsNarrationWidget::Done => 3,
        };
        frame.add(
            NarratedElementKind::Position,
            0,
            localized_gui_text_with_args(
                item_runtime,
                "narrator.position.screen",
                "Screen element %1$s out of %2$s",
                &[&position.to_string(), &active_widget_count.to_string()],
            ),
        );
        if focused_widget == Some(widget) {
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
            StatsNarrationWidget::Tabs => stats_screen_tab_narration(
                &mut frame,
                screen,
                item_runtime,
                hovered_tab.unwrap_or(screen.active_tab),
                focus == StatsScreenFocus::Tabs,
            ),
            StatsNarrationWidget::List if !screen.is_loading => {
                stats_screen_list_narration(&mut frame, stats, screen, item_runtime, hovered_row)
            }
            StatsNarrationWidget::Done => stats_screen_done_narration(
                &mut frame,
                item_runtime,
                focus == StatsScreenFocus::Done,
            ),
            StatsNarrationWidget::List => {}
        }
    } else {
        frame.add(
            NarratedElementKind::Usage,
            0,
            localized_gui_text(
                item_runtime,
                "narrator.screen.usage",
                "Use mouse cursor or Tab button to select element",
            ),
        );
    }
    frame
}

fn stats_screen_tab_narration(
    frame: &mut ScreenNarrationFrame,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    tab: usize,
    focused: bool,
) {
    let tab = tab.min(2);
    let labels = [
        screen.general_tab.as_str(),
        screen.items_tab.as_str(),
        screen.mobs_tab.as_str(),
    ];
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.tab", "%1$s tab", &[labels[tab]]),
    );
    frame.add(
        NarratedElementKind::Position,
        2,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.tab",
            "Selected tab %1$s out of %2$s",
            &[&(tab + 1).to_string(), "3"],
        ),
    );
    let tab_enabled =
        tab == 0 || (tab == 1 && screen.items_tab_enabled) || (tab == 2 && screen.mobs_tab_enabled);
    if screen.is_loading {
        frame.add(NarratedElementKind::Hint, 1, screen.loading_text.clone());
    } else if !tab_enabled {
        frame.add(
            NarratedElementKind::Hint,
            1,
            localized_gui_text(item_runtime, "gui.stats.none_found", "No statistics found"),
        );
    }
    if focused {
        frame.add(
            NarratedElementKind::Usage,
            1,
            localized_gui_text(
                item_runtime,
                "narration.tab_navigation.usage",
                "Press Ctrl and Tab to switch between tabs",
            ),
        );
    }
}

fn stats_screen_list_narration(
    frame: &mut ScreenNarrationFrame,
    stats: &StatsScreen,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    hovered_row: Option<usize>,
) {
    let list_focused = matches!(
        stats.focus,
        StatsScreenFocus::List
            | StatsScreenFocus::GeneralRow(_)
            | StatsScreenFocus::ItemHeader(_)
            | StatsScreenFocus::ItemRow(_)
            | StatsScreenFocus::MobRow(_)
    );
    if screen.active_tab == 1 {
        let hovered_header = screen.item_hovered_column;
        if let Some(row) = hovered_row {
            stats_screen_item_row_narration(frame, screen, item_runtime, row);
        } else if let Some(column) = hovered_header {
            let column = match stats.focus {
                StatsScreenFocus::ItemHeader(focused) => focused,
                _ => column,
            };
            stats_screen_item_header_narration(
                frame,
                screen,
                item_runtime,
                column,
                stats.focus == StatsScreenFocus::ItemHeader(column),
            );
        } else {
            match stats.focus {
                StatsScreenFocus::ItemHeader(column) => {
                    stats_screen_item_header_narration(frame, screen, item_runtime, column, true)
                }
                StatsScreenFocus::ItemRow(row) => {
                    stats_screen_item_row_narration(frame, screen, item_runtime, row)
                }
                _ => {}
            }
        }
        return;
    }

    let focused_row = match stats.focus {
        StatsScreenFocus::GeneralRow(row) | StatsScreenFocus::MobRow(row) => Some(row),
        _ => None,
    };
    if let Some(row_index) = hovered_row.or(focused_row) {
        if let Some(row) = row_index
            .checked_sub(screen.scroll_row)
            .and_then(|visible| screen.rows.get(visible))
        {
            let selection = match row.secondary_value.as_deref() {
                Some(secondary) => format!("{}. {}", row.value, secondary),
                None => format!("{} {}", row.label, row.value),
            };
            frame.add(
                NarratedElementKind::Title,
                1,
                localized_gui_text_with_args(
                    item_runtime,
                    "narrator.select",
                    "Selected: %1$s",
                    &[&selection],
                ),
            );
            add_stats_list_position(frame, item_runtime, row_index + 1, screen.total_rows, 2);
        }
    }
    if list_focused {
        frame.add(
            NarratedElementKind::Usage,
            1,
            localized_gui_text(
                item_runtime,
                "narration.selection.usage",
                "Press the up and down buttons to move to another entry",
            ),
        );
    }
}

fn stats_screen_item_header_narration(
    frame: &mut ScreenNarrationFrame,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    column: usize,
    focused: bool,
) {
    let column = column.min(5);
    add_stats_list_position(frame, item_runtime, 1, screen.total_rows + 1, 1);
    frame.add(
        NarratedElementKind::Title,
        3,
        localized_gui_text_with_args(
            item_runtime,
            "gui.narrate.button",
            "%1$s button",
            &[&screen.item_column_labels[column]],
        ),
    );
    frame.add(
        NarratedElementKind::Position,
        2,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.object_list",
            "Selected row element %1$s out of %2$s",
            &[&(column + 1).to_string(), "6"],
        ),
    );
    frame.add(
        NarratedElementKind::Hint,
        3,
        screen.item_column_labels[column].clone(),
    );
    frame.add(
        NarratedElementKind::Usage,
        3,
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

fn stats_screen_item_row_narration(
    frame: &mut ScreenNarrationFrame,
    screen: &HudStatsScreen,
    item_runtime: Option<&NativeItemRuntime>,
    row: usize,
) {
    let Some(item) = row
        .checked_sub(screen.scroll_row)
        .and_then(|visible| screen.item_rows.get(visible))
    else {
        return;
    };
    add_stats_list_position(frame, item_runtime, row + 2, screen.total_rows + 1, 1);
    frame.add(
        NarratedElementKind::Title,
        3,
        localized_gui_text_with_args(item_runtime, "narration.item", "Item: %1$s", &[&item.label]),
    );
}

fn add_stats_list_position(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    position: usize,
    total: usize,
    depth: u8,
) {
    if total <= 1 {
        return;
    }
    frame.add(
        NarratedElementKind::Position,
        depth,
        localized_gui_text_with_args(
            item_runtime,
            "narrator.position.list",
            "Selected list row %1$s out of %2$s",
            &[&position.to_string(), &total.to_string()],
        ),
    );
}

fn stats_screen_done_narration(
    frame: &mut ScreenNarrationFrame,
    item_runtime: Option<&NativeItemRuntime>,
    focused: bool,
) {
    let done = localized_gui_text(item_runtime, "gui.done", "Done");
    frame.add(
        NarratedElementKind::Title,
        1,
        localized_gui_text_with_args(item_runtime, "gui.narrate.button", "%1$s button", &[&done]),
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

const STATS_ITEM_COLUMN_TYPES: [&str; 6] = [
    "minecraft:mined",
    "minecraft:broken",
    "minecraft:crafted",
    "minecraft:used",
    "minecraft:picked_up",
    "minecraft:dropped",
];

struct StatsScreenData<'a> {
    amounts: BTreeMap<(&'a str, &'a str), i32>,
    items: Vec<(i32, &'a str)>,
}

impl<'a> StatsScreenData<'a> {
    fn new(world: &'a WorldStore, item_runtime: Option<&'a NativeItemRuntime>) -> Self {
        let stats = world.client_stats();
        let amounts = stats
            .identities
            .iter()
            .filter_map(|identity| {
                let amount = world.stat_value(identity.stat_type_id, identity.value_id)?;
                Some((
                    (
                        identity.stat_type_resource_id.as_deref()?,
                        identity.value_resource_id.as_deref()?,
                    ),
                    amount,
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let mut candidates = BTreeSet::new();
        for (&(stat_type, resource), &amount) in &amounts {
            let Some(column) = STATS_ITEM_COLUMN_TYPES
                .iter()
                .position(|candidate| *candidate == stat_type)
            else {
                continue;
            };
            if amount <= 0 {
                continue;
            }
            let item_resource = if column == 0 {
                item_runtime.and_then(|runtime| runtime.item_resource_id_for_block(resource))
            } else {
                Some(resource)
            };
            if let Some(resource) = item_resource.filter(|item| *item != "minecraft:air") {
                candidates.insert(resource);
            }
        }
        let items = candidates
            .into_iter()
            .filter_map(|resource| {
                let item_id = item_runtime
                    .and_then(|runtime| runtime.item_protocol_id(resource))
                    .or_else(|| {
                        bbb_protocol::item_types::vanilla_item_type_for_resource_id(resource)
                            .map(|entry| entry.id)
                    })?;
                Some((item_id, resource))
            })
            .collect();
        Self { amounts, items }
    }

    fn amount(&self, stat_type: &str, resource: &str) -> i32 {
        self.amounts
            .get(&(stat_type, resource))
            .copied()
            .unwrap_or_default()
    }

    fn mob_resources(&self) -> impl Iterator<Item = &'static str> + '_ {
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPES
            .iter()
            .filter_map(|entry| {
                (self.amount("minecraft:killed", entry.resource_id) > 0
                    || self.amount("minecraft:killed_by", entry.resource_id) > 0)
                    .then_some(entry.resource_id)
            })
    }

    fn row_counts(&self) -> [usize; 3] {
        [
            bbb_protocol::stat_types::VANILLA_CUSTOM_STATS.len(),
            self.items.len(),
            self.mob_resources().count(),
        ]
    }
}

fn stats_general_rows(
    data: &StatsScreenData<'_>,
    item_runtime: Option<&NativeItemRuntime>,
    scroll_row: usize,
    visible_rows: usize,
) -> Vec<HudStatsRow> {
    let mut rows = bbb_protocol::stat_types::VANILLA_CUSTOM_STATS
        .iter()
        .map(|entry| {
            let key = format!("stat.{}", entry.resource_id.replace(':', "."));
            (
                localized_gui_text(item_runtime, &key, entry.resource_id),
                entry,
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows.into_iter()
        .skip(scroll_row)
        .take(visible_rows)
        .map(|(label, entry)| HudStatsRow {
            label,
            value: format_vanilla_stat_value(
                entry.formatter,
                data.amount("minecraft:custom", entry.resource_id),
            ),
            secondary_value: None,
        })
        .collect()
}

fn stats_mob_rows(
    data: &StatsScreenData<'_>,
    item_runtime: Option<&NativeItemRuntime>,
    scroll_row: usize,
    visible_rows: usize,
) -> Vec<HudStatsRow> {
    let mut rows = data
        .mob_resources()
        .map(|resource| {
            let key = format!("entity.{}", resource.replace(':', "."));
            (localized_gui_text(item_runtime, &key, resource), resource)
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    rows.into_iter()
        .skip(scroll_row)
        .take(visible_rows)
        .map(|(name, resource)| {
            let kills = data.amount("minecraft:killed", resource);
            let killed_by = data.amount("minecraft:killed_by", resource);
            let kills_text = if kills == 0 {
                localized_gui_text_with_args(
                    item_runtime,
                    "stat_type.minecraft.killed.none",
                    "You have never killed %1$s",
                    &[&name],
                )
            } else {
                localized_gui_text_with_args(
                    item_runtime,
                    "stat_type.minecraft.killed",
                    "You killed %1$s %2$s",
                    &[&format_grouped_integer(kills), &name],
                )
            };
            let killed_by_text = if killed_by == 0 {
                localized_gui_text_with_args(
                    item_runtime,
                    "stat_type.minecraft.killed_by.none",
                    "You have never been killed by %1$s",
                    &[&name],
                )
            } else {
                localized_gui_text_with_args(
                    item_runtime,
                    "stat_type.minecraft.killed_by",
                    "%1$s killed you %2$s time(s)",
                    &[&name, &format_grouped_integer(killed_by)],
                )
            };
            HudStatsRow {
                label: name,
                value: kills_text,
                secondary_value: Some(killed_by_text),
            }
        })
        .collect()
}

struct StatsItemRowCandidate<'a> {
    item_id: i32,
    resource_id: &'a str,
    raw_values: [Option<i32>; 6],
}

fn stats_item_rows<'a>(
    effective_sort: Option<(usize, i8)>,
    data: &StatsScreenData<'a>,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<StatsItemRowCandidate<'a>> {
    let mut rows = data
        .items
        .iter()
        .map(|&(item_id, resource_id)| {
            let mined = item_runtime
                .and_then(|runtime| runtime.primary_block_resource_id_for_item(resource_id))
                .map(|block| data.amount(STATS_ITEM_COLUMN_TYPES[0], block));
            let raw_values = std::array::from_fn(|column| {
                if column == 0 {
                    mined
                } else {
                    Some(data.amount(STATS_ITEM_COLUMN_TYPES[column], resource_id))
                }
            });
            StatsItemRowCandidate {
                item_id,
                resource_id,
                raw_values,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by_key(|candidate| candidate.item_id);
    if let Some((column, order)) = effective_sort {
        rows.sort_by(|left, right| {
            let ordering = left.raw_values[column]
                .unwrap_or(-1)
                .cmp(&right.raw_values[column].unwrap_or(-1))
                .then_with(|| left.item_id.cmp(&right.item_id));
            if order < 0 {
                ordering.reverse()
            } else {
                ordering
            }
        });
    }
    rows
}

fn stats_item_label(item_runtime: Option<&NativeItemRuntime>, resource_id: &str) -> String {
    item_runtime
        .map(|runtime| runtime.localized_item_name(resource_id))
        .unwrap_or_else(|| resource_id.to_owned())
}

pub(super) fn stats_screen_viewport_height(
    surface_size: winit::dpi::PhysicalSize<u32>,
    row_height: i32,
) -> i32 {
    const LIST_TOP: i32 = bbb_client_render::HUD_STATS_LIST_TOP;
    const FOOTER_HEIGHT: i32 = 33;
    let height = i32::try_from(surface_size.height).unwrap_or(i32::MAX);
    (height - FOOTER_HEIGHT - LIST_TOP).max(row_height)
}

pub(super) fn stats_screen_scroller_height(
    viewport_height: i32,
    content_height: i32,
    max_scroll_amount: i32,
) -> i32 {
    if max_scroll_amount == 0 || content_height <= 0 {
        return 0;
    }
    viewport_height
        .saturating_mul(viewport_height)
        .checked_div(content_height)
        .unwrap_or(viewport_height)
        .clamp(32, (viewport_height - 8).max(32))
}

pub(super) fn stats_screen_first_data_row(
    active_tab: usize,
    scroll_amount: i32,
    row_height: i32,
) -> usize {
    let entry =
        usize::try_from(scroll_amount.saturating_sub(2).max(0) / row_height).unwrap_or_default();
    if active_tab == 1 {
        entry.saturating_sub(1)
    } else {
        entry
    }
}

pub(super) fn stats_screen_first_data_row_offset(
    active_tab: usize,
    scroll_row: usize,
    scroll_amount: i32,
    row_height: i32,
) -> i32 {
    let header_height = i32::from(active_tab == 1) * row_height;
    2i32.saturating_add(header_height)
        .saturating_add(
            i32::try_from(scroll_row)
                .unwrap_or(i32::MAX / row_height)
                .saturating_mul(row_height),
        )
        .saturating_sub(scroll_amount)
}

pub(super) fn stats_screen_projected_row_count(
    remaining_rows: usize,
    viewport_height: i32,
    first_row_offset: i32,
    row_height: i32,
) -> usize {
    if remaining_rows == 0 || first_row_offset >= viewport_height {
        return 0;
    }
    let visible_height = viewport_height.saturating_sub(first_row_offset);
    let count = visible_height
        .saturating_add(row_height - 1)
        .checked_div(row_height)
        .unwrap_or_default();
    usize::try_from(count)
        .unwrap_or(remaining_rows)
        .min(remaining_rows)
}

pub(super) fn format_vanilla_stat_value(formatter: &str, value: i32) -> String {
    match formatter {
        "DIVIDE_BY_TEN" => format!("{:.2}", f64::from(value) * 0.1),
        "DISTANCE" => {
            let meters = f64::from(value) / 100.0;
            let kilometers = meters / 1000.0;
            if kilometers > 0.5 {
                format!("{kilometers:.2} km")
            } else if meters > 0.5 {
                format!("{meters:.2} m")
            } else {
                format!("{value} cm")
            }
        }
        "TIME" => {
            let seconds = f64::from(value) / 20.0;
            let minutes = seconds / 60.0;
            let hours = minutes / 60.0;
            let days = hours / 24.0;
            let years = days / 365.0;
            if years > 0.5 {
                format!("{years:.2} y")
            } else if days > 0.5 {
                format!("{days:.2} d")
            } else if hours > 0.5 {
                format!("{hours:.2} h")
            } else if minutes > 0.5 {
                format!("{minutes:.2} min")
            } else {
                format!("{} s", java_double_string(seconds))
            }
        }
        _ => format_grouped_integer(value),
    }
}

fn java_double_string(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

fn format_grouped_integer(value: i32) -> String {
    let negative = value < 0;
    let digits = i64::from(value).abs().to_string();
    let mut grouped =
        String::with_capacity(digits.len() + digits.len() / 3 + usize::from(negative));
    if negative {
        grouped.push('-');
    }
    let first = digits.len() % 3;
    if first != 0 {
        grouped.push_str(&digits[..first]);
        if digits.len() > first {
            grouped.push(',');
        }
    }
    for (index, chunk) in digits.as_bytes()[first..].chunks(3).enumerate() {
        if index > 0 {
            grouped.push(',');
        }
        grouped.push_str(std::str::from_utf8(chunk).expect("decimal digits are valid UTF-8"));
    }
    grouped
}
