use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    HudBookHitTarget, HudBookInteraction, HudBookMode, HudBookScreen, HudComponentTextRun,
    HudStyledTextRun, HudTextStyle,
};
use bbb_client_world::{BookScreenState, WorldStore};
use bbb_protocol::{ComponentClickEvent, ComponentStyle, StyledTextRun};

use crate::{
    component_text::{localized_component_runs, localized_or_fallback},
    input::{BookInputMode, BookScreen, ClientInputState},
};

const BOOK_TEXT_COLOR: u32 = 0x00_00_00;

pub(super) fn hud_book_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudBookScreen> {
    let text_filtering_enabled = input.client_text_filtering_enabled();
    let Some(screen) = input.book_screen() else {
        if !matches!(
            input.current_screen(),
            Some(crate::input::Screen::Container(_))
        ) {
            return None;
        }
        let lectern = world.lectern_book_view()?;
        let page_count = lectern.pages.len().max(1);
        let page = lectern.current_page.min(page_count.saturating_sub(1));
        let component = lectern
            .pages
            .get(page)
            .map(|page| page.component_for_filter(text_filtering_enabled));
        let source_runs = book_component_runs(component, item_runtime);
        let (runs, interactions) = project_component_runs(world, item_runtime, &source_runs);
        return Some(
            projection_common(
                input,
                item_runtime,
                lectern.revision,
                page,
                page_count,
                text_filtering_enabled,
                lectern.may_build,
            )
            .finish(
                HudBookMode::Lectern,
                runs,
                interactions,
                0,
                0,
                false,
                String::new(),
                String::new(),
                String::new(),
                false,
            ),
        );
    };
    let book = &screen.book;
    let page_count = book.page_count().max(1);
    let page = book.current_page().min(page_count.saturating_sub(1));
    let common = projection_common(
        input,
        item_runtime,
        book.generation(),
        page,
        page_count,
        text_filtering_enabled,
        false,
    );
    let mut projected = match book {
        BookScreenState::View(view) => {
            let component = view
                .pages
                .get(page)
                .map(|page| page.component_for_filter(text_filtering_enabled));
            let source_runs = book_component_runs(component, item_runtime);
            let (runs, interactions) = project_component_runs(world, item_runtime, &source_runs);
            common.finish(
                HudBookMode::View,
                runs,
                interactions,
                0,
                0,
                false,
                String::new(),
                String::new(),
                String::new(),
                false,
            )
        }
        BookScreenState::Edit(edit) => {
            let text = edit
                .pages
                .get(page)
                .map_or("", bbb_client_world::BookEditorText::display);
            let (cursor, selection) = book_editor_selection(screen);
            common.finish(
                HudBookMode::Edit,
                vec![HudComponentTextRun {
                    visual: HudStyledTextRun {
                        shadow_color: None,
                        text: text.to_string(),
                        style: HudTextStyle::default(),
                        color: Some(BOOK_TEXT_COLOR),
                    },
                    font: None,
                }],
                Vec::new(),
                cursor,
                selection,
                book_cursor_visible(screen),
                String::new(),
                String::new(),
                String::new(),
                false,
            )
        }
        BookScreenState::Sign(sign) => {
            let (cursor, selection) = book_editor_selection(screen);
            common.finish(
                HudBookMode::Sign,
                vec![HudComponentTextRun {
                    visual: HudStyledTextRun {
                        shadow_color: None,
                        text: sign.title.display().to_string(),
                        style: HudTextStyle::default(),
                        color: Some(BOOK_TEXT_COLOR),
                    },
                    font: None,
                }],
                Vec::new(),
                cursor,
                selection,
                book_cursor_visible(screen),
                localized_or_fallback(item_runtime, "book.editTitle", "Enter Book Title:"),
                localized_with_args(item_runtime, "book.byAuthor", "by %1$s", &[&sign.author]),
                localized_or_fallback(
                    item_runtime,
                    "book.finalizeWarning",
                    "Note! When you sign the book, it will no longer be editable.",
                ),
                sign.can_finalize_with_button(),
            )
        }
    };
    projected.editor_text = match book {
        BookScreenState::Edit(edit) => edit
            .pages
            .get(page)
            .map(|text| text.units().to_vec())
            .unwrap_or_default(),
        BookScreenState::Sign(sign) => sign.title.units().to_vec(),
        BookScreenState::View(_) => Vec::new(),
    };
    projected.editor_display_start = screen.input.as_ref().map_or(0, |state| state.display_start);
    projected.preedit = input
        .book_ime_preedit_hud_state()
        .map(crate::input::ImePreeditHudState::into_hud_ime_preedit);
    Some(projected)
}

pub(crate) fn hud_book_component_click_event(
    world: &WorldStore,
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    target: HudBookHitTarget,
) -> Option<ComponentClickEvent> {
    if target.resource_generation != item_runtime.map_or(0, NativeItemRuntime::resource_generation)
        || target.resources_loaded != item_runtime.is_some()
        || target.text_filtering_enabled != input.client_text_filtering_enabled()
    {
        return None;
    }
    let source_runs = match target.mode {
        HudBookMode::View => {
            let BookScreenState::View(screen) = &input.book_screen()?.book else {
                return None;
            };
            if target.generation != screen.generation || target.page != screen.current_page {
                return None;
            }
            book_component_runs(
                Some(
                    screen
                        .pages
                        .get(screen.current_page)?
                        .component_for_filter(target.text_filtering_enabled),
                ),
                item_runtime,
            )
        }
        HudBookMode::Lectern => {
            if !matches!(
                input.current_screen(),
                Some(crate::input::Screen::Container(_))
            ) {
                return None;
            }
            let lectern = world.lectern_book_view()?;
            if target.generation != lectern.revision || target.page != lectern.current_page {
                return None;
            }
            book_component_runs(
                Some(
                    lectern
                        .pages
                        .get(lectern.current_page)?
                        .component_for_filter(target.text_filtering_enabled),
                ),
                item_runtime,
            )
        }
        HudBookMode::Edit | HudBookMode::Sign => return None,
    };
    source_runs
        .get(target.source_run_index)?
        .style
        .click_event
        .clone()
}

fn book_component_runs(
    component: Option<&bbb_protocol::RichTextComponent>,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<StyledTextRun> {
    component.map_or_else(Vec::new, |component| {
        localized_component_runs(
            component,
            &ComponentStyle {
                color: Some(BOOK_TEXT_COLOR),
                shadow_color: Some(0),
                ..ComponentStyle::default()
            },
            item_runtime,
        )
    })
}

fn projection_common(
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    generation: u64,
    page: usize,
    page_count: usize,
    text_filtering_enabled: bool,
    may_build: bool,
) -> BookProjectionCommon {
    BookProjectionCommon {
        generation,
        page,
        page_count,
        resource_generation: item_runtime.map_or(0, NativeItemRuntime::resource_generation),
        resources_loaded: item_runtime.is_some(),
        text_filtering_enabled,
        advanced_item_tooltips: input.debug_advanced_item_tooltips(),
        cursor_position: input.inventory_cursor_position(),
        page_indicator: localized_with_args(
            item_runtime,
            "book.pageIndicator",
            "Page %1$s of %2$s",
            &[&(page + 1).to_string(), &page_count.to_string()],
        ),
        done: localized_or_fallback(item_runtime, "gui.done", "Done"),
        take_book: localized_or_fallback(item_runtime, "lectern.take_book", "Take Book"),
        may_build,
        sign: localized_or_fallback(item_runtime, "book.signButton", "Sign"),
        finalize: localized_or_fallback(item_runtime, "book.finalizeButton", "Sign and Close"),
        cancel: localized_or_fallback(item_runtime, "gui.cancel", "Cancel"),
    }
}

struct BookProjectionCommon {
    generation: u64,
    page: usize,
    page_count: usize,
    resource_generation: u64,
    resources_loaded: bool,
    text_filtering_enabled: bool,
    advanced_item_tooltips: bool,
    cursor_position: Option<(i32, i32)>,
    page_indicator: String,
    done: String,
    take_book: String,
    may_build: bool,
    sign: String,
    finalize: String,
    cancel: String,
}

impl BookProjectionCommon {
    #[allow(clippy::too_many_arguments)]
    fn finish(
        self,
        mode: HudBookMode,
        runs: Vec<HudComponentTextRun>,
        interactions: Vec<HudBookInteraction>,
        cursor: usize,
        selection: usize,
        cursor_visible: bool,
        title: String,
        author: String,
        warning: String,
        finalize_enabled: bool,
    ) -> HudBookScreen {
        HudBookScreen {
            editor_text: Vec::new(),
            editor_display_start: 0,
            generation: self.generation,
            page: self.page,
            page_count: self.page_count,
            resource_generation: self.resource_generation,
            resources_loaded: self.resources_loaded,
            text_filtering_enabled: self.text_filtering_enabled,
            advanced_item_tooltips: self.advanced_item_tooltips,
            mode,
            runs,
            interactions,
            cursor,
            selection,
            cursor_visible,
            cursor_position: self.cursor_position,
            page_indicator: self.page_indicator,
            title,
            author,
            warning,
            done: self.done,
            take_book: self.take_book,
            may_build: self.may_build,
            sign: self.sign,
            finalize: self.finalize,
            cancel: self.cancel,
            finalize_enabled,
            preedit: None,
        }
    }
}

fn project_component_runs(
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
    source_runs: &[StyledTextRun],
) -> (Vec<HudComponentTextRun>, Vec<HudBookInteraction>) {
    let mut char_offset = 0usize;
    let mut runs = Vec::with_capacity(source_runs.len());
    let mut interactions = Vec::new();
    for (source_run_index, source) in source_runs.iter().enumerate() {
        let mut visual = super::component_hover::hud_component_run(source.clone());
        visual
            .visual
            .text
            .retain(|ch| ch == '\n' || !ch.is_control());
        let start_char = char_offset;
        char_offset = char_offset.saturating_add(visual.visual.text.chars().count());
        if (source.style.click_event.is_some() || source.style.hover_event.is_some())
            && start_char < char_offset
        {
            interactions.push(HudBookInteraction {
                start_char,
                end_char: char_offset,
                source_run_index,
                clickable: source.style.click_event.is_some(),
                tooltip: super::component_hover::hud_component_hover_tooltip(
                    world,
                    item_runtime,
                    source.style.hover_event.as_ref(),
                ),
            });
        }
        runs.push(visual);
    }
    (runs, interactions)
}

fn book_editor_selection(screen: &BookScreen) -> (usize, usize) {
    screen
        .input
        .as_ref()
        .filter(|state| match (state.mode, &screen.book) {
            (BookInputMode::Edit { page }, BookScreenState::Edit(edit)) => {
                page == edit.current_page
            }
            (BookInputMode::Sign, BookScreenState::Sign(_)) => true,
            _ => false,
        })
        .map_or((0, 0), |state| (state.cursor, state.selection))
}

fn book_cursor_visible(screen: &BookScreen) -> bool {
    screen.input.as_ref().is_some_and(|state| {
        crate::input::text_field::caret_visible(
            state.focused_at,
            crate::input::text_field::clock_millis(),
        )
    })
}

fn localized_with_args(
    item_runtime: Option<&NativeItemRuntime>,
    key: &str,
    fallback: &str,
    arguments: &[&str],
) -> String {
    let mut value = localized_or_fallback(item_runtime, key, fallback);
    for (index, argument) in arguments.iter().enumerate() {
        value = value.replace(&format!("%{}$s", index + 1), argument);
    }
    let mut sequential = arguments.iter();
    while value.contains("%s") {
        let Some(argument) = sequential.next() else {
            break;
        };
        value = value.replacen("%s", argument, 1);
    }
    value
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        menu_types::VANILLA_MENU_TYPE_LECTERN_ID,
        packets::{
            ContainerSetContent, ContainerSetData, InteractionHand, ItemStackSummary, OpenBook,
            OpenScreen, SetPlayerInventory, WrittenBookContentSummary,
        },
        NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
    };

    use super::*;
    use crate::input::Screen;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn book_editor_projection_preserves_raw_units_and_unit_cursor() {
        let mut world = WorldStore::new();
        let mut item = ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_WRITABLE_BOOK_ID),
            count: 1,
            component_patch: Default::default(),
        };
        item.component_patch.writable_book_pages = vec![Default::default()];
        world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item });
        let mut input = ClientInputState::default();
        let mut book = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();
        let units = vec![0xd83d, 0xde00, 0xd800, 65];
        assert!(book.replace_page(units.clone()));
        input.set_screen_for_test(Screen::Book(BookScreen::new(book)));
        let screen = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(screen.editor_text, units);
        assert_eq!(screen.cursor, 4);
        assert_eq!(screen.selection, 4);
        let mut book = input.book_screen().unwrap().book.clone();
        assert!(book.begin_sign(String::new()));
        assert!(book.replace_title(vec![0xd83d, 0xde00, 0xd800, 65]));
        input.set_screen_for_test(Screen::Book(BookScreen::new(book)));
        let screen = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(screen.editor_text, [0xd83d, 0xde00, 0xd800, 65]);
        assert_eq!((screen.cursor, screen.selection), (4, 4));
        let mut glyphs = bbb_client_render::HudFontGlyphMap::new();
        glyphs.set_missing_glyph(bbb_client_render::HudAsciiGlyph::empty(80.0));
        input.handle_book_screen_text("B", &glyphs);
        let screen = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(
            screen.editor_display_start,
            input.book_input_state().unwrap().display_start
        );
        assert!(screen.editor_display_start > 0);
        for (unit, enabled) in [
            (0x1c, false),
            (0xa0, false),
            (0x2007, false),
            (0x85, true),
            (0xd800, true),
        ] {
            input
                .book_screen_mut()
                .unwrap()
                .book
                .replace_title(vec![unit]);
            let screen = hud_book_screen(&input, &world, None).unwrap();
            assert_eq!(screen.finalize_enabled, enabled, "{unit:x}");
        }
    }

    #[test]
    fn book_page_default_shadow_is_inherited_but_explicit_shadow_wins() {
        let component = RichTextComponent::try_from_component_nbt(
            bbb_protocol::json_value_to_nbt_summary(&serde_json::json!({
                "text": "plain", "extra": [
                    {"text": "shadow", "shadow_color": -65536},
                    {"text": "plain again"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let world = WorldStore::default();
        let source = book_component_runs(Some(&component), None);
        let (runs, _) = project_component_runs(&world, None, &source);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].visual.shadow_color, Some(0));
        assert_eq!(runs[1].visual.shadow_color, Some(0xffff0000));
        assert_eq!(runs[2].visual.shadow_color, Some(0));
        assert!(runs
            .iter()
            .all(|run| run.visual.color == Some(BOOK_TEXT_COLOR)));
    }

    #[test]
    fn rich_book_projection_preserves_style_interaction_and_filtered_page() {
        let raw = NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("raw")),
            entry("bold", NbtSummaryValue::Byte(1)),
            entry("font", NbtSummaryValue::string("example:book")),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::string("change_page")),
                    entry("page", NbtSummaryValue::Int(2)),
                ]),
            ),
            entry(
                "hover_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::string("show_text")),
                    entry(
                        "value",
                        NbtSummaryValue::Compound(vec![
                            entry("text", NbtSummaryValue::string("tip")),
                            entry("font", NbtSummaryValue::string("example:tooltip")),
                        ]),
                    ),
                ]),
            ),
        ]);
        let filtered = NbtSummaryValue::string("filtered");
        let mut input = ClientInputState::default();
        let world = written_book_world(&mut input, vec![raw], vec![Some(filtered)]);
        input.set_client_text_filtering_enabled(false);
        let projected = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(projected.runs[0].visual.text, "raw");
        assert_eq!(projected.runs[0].visual.shadow_color, Some(0));
        assert!(projected.runs[0].visual.style.bold);
        assert_eq!(projected.runs[0].font.as_deref(), Some("example:book"));
        assert_eq!(projected.interactions[0].source_run_index, 0);
        assert!(projected.interactions[0].clickable);
        let tooltip = projected.interactions[0].tooltip.as_ref().unwrap();
        assert_eq!(tooltip.lines[0].runs[0].visual.text, "tip");
        assert_eq!(
            tooltip.lines[0].runs[0].font.as_deref(),
            Some("example:tooltip")
        );
        let BookScreenState::View(screen) = &input.book_screen().unwrap().book else {
            panic!("written book must remain a view");
        };
        assert_eq!(
            screen.pages[0].raw.styled()[0].style.font.as_deref(),
            Some("example:book")
        );

        input.set_debug_advanced_item_tooltips(true);
        assert!(
            hud_book_screen(&input, &world, None)
                .unwrap()
                .advanced_item_tooltips
        );
        input.set_client_text_filtering_enabled(true);
        let projected = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(projected.runs[0].visual.text, "filtered");
        assert!(projected.interactions.is_empty());
    }

    #[test]
    fn lectern_projection_preserves_rich_filtering_and_rejects_stale_targets() {
        let raw = NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("raw")),
            entry("bold", NbtSummaryValue::Byte(1)),
            entry("font", NbtSummaryValue::string("example:book")),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::string("change_page")),
                    entry("page", NbtSummaryValue::Int(2)),
                ]),
            ),
        ]);
        let mut item = ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: Default::default(),
        };
        item.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: None,
            author: "Alex".into(),
            generation: 0,
            pages: vec!["raw".to_string(), "second".to_string()],
            page_filters: vec![Some("filtered".to_string()), None],
            resolved: true,
        });
        item.component_patch.written_book_pages_nbt =
            vec![Some(raw), Some(NbtSummaryValue::string("second"))];
        item.component_patch.written_book_page_filters_nbt =
            vec![Some(NbtSummaryValue::string("filtered")), None];
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
            title: RichTextComponent::plain("Lectern"),
        });
        assert!(world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 1,
            items: vec![item],
            carried_item: ItemStackSummary::empty(),
        }));

        let mut input = ClientInputState::default();
        input.apply_screen_requests(
            &mut world,
            &mut crate::control::NetCounters::default(),
            &None,
        );
        let screen = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(screen.mode, HudBookMode::Lectern);
        assert_eq!(screen.runs[0].visual.text, "raw");
        assert!(screen.runs[0].visual.style.bold);
        assert_eq!(screen.runs[0].font.as_deref(), Some("example:book"));
        assert!(screen.may_build);
        let target = HudBookHitTarget {
            mode: screen.mode,
            generation: screen.generation,
            page: screen.page,
            resource_generation: screen.resource_generation,
            resources_loaded: screen.resources_loaded,
            text_filtering_enabled: screen.text_filtering_enabled,
            source_run_index: 0,
        };
        assert_eq!(
            hud_book_component_click_event(&world, &input, None, target),
            Some(ComponentClickEvent::ChangePage { page: 2 })
        );

        input.set_client_text_filtering_enabled(true);
        assert_eq!(
            hud_book_component_click_event(&world, &input, None, target),
            None
        );
        let filtered = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(filtered.runs[0].visual.text, "filtered");
        assert!(filtered.interactions.is_empty());

        input.set_client_text_filtering_enabled(false);
        assert!(world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: 0,
            value: 0,
        }));
        assert_eq!(
            hud_book_component_click_event(&world, &input, None, target),
            None
        );
    }

    #[test]
    fn click_event_revalidates_generation_page_and_source_run() {
        let page = NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::string("next")),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::string("change_page")),
                    entry("page", NbtSummaryValue::Int(2)),
                ]),
            ),
        ]);
        let mut input = ClientInputState::default();
        let world = written_book_world(
            &mut input,
            vec![page, NbtSummaryValue::string("two")],
            vec![None, None],
        );
        let screen = hud_book_screen(&input, &world, None).unwrap();
        let target = HudBookHitTarget {
            mode: HudBookMode::View,
            generation: screen.generation,
            page: 0,
            resource_generation: 0,
            resources_loaded: false,
            text_filtering_enabled: false,
            source_run_index: 0,
        };
        assert_eq!(
            hud_book_component_click_event(&world, &input, None, target),
            Some(ComponentClickEvent::ChangePage { page: 2 })
        );
        let stale = HudBookHitTarget {
            generation: target.generation + 1,
            ..target
        };
        assert_eq!(
            hud_book_component_click_event(&world, &input, None, stale),
            None
        );
    }

    fn written_book_world(
        input: &mut ClientInputState,
        pages_nbt: Vec<NbtSummaryValue>,
        page_filters_nbt: Vec<Option<NbtSummaryValue>>,
    ) -> WorldStore {
        let mut world = WorldStore::new();
        let pages = pages_nbt
            .iter()
            .map(|page| {
                RichTextComponent::try_from_component_nbt(page.clone())
                    .unwrap()
                    .text()
                    .to_string()
            })
            .collect::<Vec<_>>();
        let page_filters = page_filters_nbt
            .iter()
            .map(|page| {
                page.as_ref().map(|page| {
                    RichTextComponent::try_from_component_nbt(page.clone())
                        .unwrap()
                        .text()
                        .to_string()
                })
            })
            .collect();
        let mut item = ItemStackSummary {
            item_id: Some(42),
            count: 1,
            component_patch: Default::default(),
        };
        item.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: None,
            author: "Alex".into(),
            generation: 0,
            pages,
            page_filters,
            resolved: true,
        });
        item.component_patch.written_book_pages_nbt = pages_nbt.into_iter().map(Some).collect();
        item.component_patch.written_book_page_filters_nbt = page_filters_nbt;
        assert!(world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item }));
        world.apply_open_book(OpenBook {
            hand: InteractionHand::MainHand,
        });
        input.apply_screen_requests(&mut world, &mut Default::default(), &None);
        world
    }
}

#[cfg(test)]
mod ordinal_tests {
    use super::*;
    #[test]
    fn book_projection_preserves_target_after_u16_ordinal_boundary() {
        let mut source = vec![
            StyledTextRun {
                text: "a".into(),
                style: Default::default()
            };
            65537
        ];
        source[65536].style.click_event =
            Some(bbb_protocol::ComponentClickEvent::ChangePage { page: 2 });
        let (_, targets) = project_component_runs(&WorldStore::new(), None, &source);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].source_run_index, 65536);
        assert_eq!(targets[0].start_char, 65536);
    }
}
