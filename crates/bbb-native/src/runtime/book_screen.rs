use bbb_item_model::NativeItemRuntime;
use bbb_protocol::{ComponentClickEvent, ComponentHoverEvent, ComponentStyle, StyledTextRun};
use bbb_renderer::{
    HudBookHitTarget, HudBookInteraction, HudBookMode, HudBookScreen, HudStyledTextRun,
    HudTextStyle,
};
use bbb_world::{BookScreenState, WorldStore};

use crate::{
    component_text::{localized_component_runs, localized_or_fallback},
    input::{BookInputMode, ClientInputState},
};

const BOOK_TEXT_COLOR: u32 = 0x00_00_00;

pub(super) fn hud_book_screen(
    input: &ClientInputState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> Option<HudBookScreen> {
    let book = world.current_book()?;
    let page_count = book.page_count().max(1);
    let page = book.current_page().min(page_count.saturating_sub(1));
    let cursor_position = input.inventory_cursor_position();
    let resource_generation = item_runtime.map_or(0, NativeItemRuntime::resource_generation);
    let resources_loaded = item_runtime.is_some();
    let page_indicator = localized_with_args(
        item_runtime,
        "book.pageIndicator",
        "Page %1$s of %2$s",
        &[&(page + 1).to_string(), &page_count.to_string()],
    );
    let common = BookProjectionCommon {
        generation: book.generation(),
        page,
        page_count,
        resource_generation,
        resources_loaded,
        cursor_position,
        page_indicator,
        done: localized_or_fallback(item_runtime, "gui.done", "Done"),
        sign: localized_or_fallback(item_runtime, "book.signButton", "Sign"),
        finalize: localized_or_fallback(item_runtime, "book.finalizeButton", "Sign and Close"),
        cancel: localized_or_fallback(item_runtime, "gui.cancel", "Cancel"),
    };
    Some(match book {
        BookScreenState::View(screen) => {
            let component = screen
                .pages
                .get(page)
                .map(|page| page.component_for_filter(input.client_text_filtering_enabled()));
            let source_runs = component.map_or_else(Vec::new, |component| {
                localized_component_runs(
                    component,
                    &ComponentStyle {
                        color: Some(BOOK_TEXT_COLOR),
                        ..ComponentStyle::default()
                    },
                    item_runtime,
                )
            });
            let (runs, interactions) = project_component_runs(&source_runs);
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
        BookScreenState::Edit(screen) => {
            let text = screen.pages.get(page).map_or("", String::as_str);
            let (cursor, selection) = book_editor_selection(input, book);
            common.finish(
                HudBookMode::Edit,
                vec![HudStyledTextRun {
                    text: text.to_string(),
                    style: HudTextStyle::default(),
                    color: Some(BOOK_TEXT_COLOR),
                }],
                Vec::new(),
                cursor,
                selection,
                book_cursor_visible(world),
                String::new(),
                String::new(),
                String::new(),
                false,
            )
        }
        BookScreenState::Sign(screen) => {
            let (cursor, selection) = book_editor_selection(input, book);
            common.finish(
                HudBookMode::Sign,
                vec![HudStyledTextRun {
                    text: screen.title.clone(),
                    style: HudTextStyle::default(),
                    color: Some(BOOK_TEXT_COLOR),
                }],
                Vec::new(),
                cursor,
                selection,
                book_cursor_visible(world),
                localized_or_fallback(item_runtime, "book.editTitle", "Enter Book Title:"),
                localized_with_args(item_runtime, "book.byAuthor", "by %1$s", &[&screen.author]),
                localized_or_fallback(
                    item_runtime,
                    "book.finalizeWarning",
                    "Note! When you sign the book, it will no longer be editable.",
                ),
                !screen.title.trim().is_empty(),
            )
        }
    })
}

pub(crate) fn hud_book_component_click_event(
    world: &WorldStore,
    input: &ClientInputState,
    item_runtime: Option<&NativeItemRuntime>,
    target: HudBookHitTarget,
) -> Option<ComponentClickEvent> {
    let BookScreenState::View(screen) = world.current_book()? else {
        return None;
    };
    if target.generation != screen.generation
        || target.page != screen.current_page
        || target.resource_generation
            != item_runtime.map_or(0, NativeItemRuntime::resource_generation)
        || target.resources_loaded != item_runtime.is_some()
    {
        return None;
    }
    let component = screen
        .pages
        .get(screen.current_page)?
        .component_for_filter(input.client_text_filtering_enabled());
    localized_component_runs(
        component,
        &ComponentStyle {
            color: Some(BOOK_TEXT_COLOR),
            ..ComponentStyle::default()
        },
        item_runtime,
    )
    .get(usize::from(target.source_run_index))?
    .style
    .click_event
    .clone()
}

struct BookProjectionCommon {
    generation: u64,
    page: usize,
    page_count: usize,
    resource_generation: u64,
    resources_loaded: bool,
    cursor_position: Option<(i32, i32)>,
    page_indicator: String,
    done: String,
    sign: String,
    finalize: String,
    cancel: String,
}

impl BookProjectionCommon {
    #[allow(clippy::too_many_arguments)]
    fn finish(
        self,
        mode: HudBookMode,
        runs: Vec<HudStyledTextRun>,
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
            generation: self.generation,
            page: self.page,
            page_count: self.page_count,
            resource_generation: self.resource_generation,
            resources_loaded: self.resources_loaded,
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
            sign: self.sign,
            finalize: self.finalize,
            cancel: self.cancel,
            finalize_enabled,
        }
    }
}

fn project_component_runs(
    source_runs: &[StyledTextRun],
) -> (Vec<HudStyledTextRun>, Vec<HudBookInteraction>) {
    let mut char_offset = 0usize;
    let mut runs = Vec::with_capacity(source_runs.len());
    let mut interactions = Vec::new();
    for (source_run_index, source) in source_runs.iter().enumerate() {
        let mut visual = hud_run_from_component(source);
        visual.text.retain(|ch| ch == '\n' || !ch.is_control());
        let start_char = char_offset;
        char_offset = char_offset.saturating_add(visual.text.chars().count());
        if (source.style.click_event.is_some() || source.style.hover_event.is_some())
            && start_char < char_offset
        {
            if let Ok(target_index) = u16::try_from(source_run_index) {
                interactions.push(HudBookInteraction {
                    start_char,
                    end_char: char_offset,
                    source_run_index: target_index,
                    clickable: source.style.click_event.is_some(),
                    tooltip_runs: hover_runs(source.style.hover_event.as_ref()),
                });
            }
        }
        runs.push(visual);
    }
    (runs, interactions)
}

fn hud_run_from_component(run: &StyledTextRun) -> HudStyledTextRun {
    HudStyledTextRun {
        text: run.text.clone(),
        style: HudTextStyle {
            bold: run.style.bold == Some(true),
            italic: run.style.italic == Some(true),
            underlined: run.style.underlined == Some(true),
            strikethrough: run.style.strikethrough == Some(true),
            obfuscated: run.style.obfuscated == Some(true),
        },
        color: run.style.color,
    }
}

fn hover_runs(hover: Option<&ComponentHoverEvent>) -> Vec<HudStyledTextRun> {
    match hover {
        Some(ComponentHoverEvent::ShowText { value }) => {
            value.iter().map(hud_run_from_component).collect()
        }
        Some(ComponentHoverEvent::ShowItem { id, count, .. }) => {
            vec![HudStyledTextRun::plain(if *count == 1 {
                id.clone()
            } else {
                format!("{id} x{count}")
            })]
        }
        Some(ComponentHoverEvent::ShowEntity { id, name, .. }) => name
            .as_ref()
            .filter(|name| !name.is_empty())
            .map(|name| name.iter().map(hud_run_from_component).collect())
            .unwrap_or_else(|| vec![HudStyledTextRun::plain(id)]),
        None => Vec::new(),
    }
}

fn book_editor_selection(input: &ClientInputState, book: &BookScreenState) -> (usize, usize) {
    input
        .book_input_state()
        .filter(|state| {
            state.generation == book.generation()
                && match (state.mode, book) {
                    (BookInputMode::Edit { page }, BookScreenState::Edit(screen)) => {
                        page == screen.current_page
                    }
                    (BookInputMode::Sign, BookScreenState::Sign(_)) => true,
                    _ => false,
                }
        })
        .map_or((0, 0), |state| (state.cursor, state.selection))
}

fn book_cursor_visible(world: &WorldStore) -> bool {
    world.client_chat().gui_ticks / 6 % 2 == 0
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
        packets::{
            InteractionHand, ItemStackSummary, OpenBook, SetPlayerInventory,
            WrittenBookContentSummary,
        },
        NbtSummaryEntry, NbtSummaryValue, RichTextComponent,
    };

    use super::*;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn rich_book_projection_preserves_style_interaction_and_filtered_page() {
        let raw = NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::String("raw".to_string())),
            entry("bold", NbtSummaryValue::Byte(1)),
            entry("font", NbtSummaryValue::String("example:book".to_string())),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::String("change_page".to_string())),
                    entry("page", NbtSummaryValue::Int(2)),
                ]),
            ),
            entry(
                "hover_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::String("show_text".to_string())),
                    entry("value", NbtSummaryValue::String("tip".to_string())),
                ]),
            ),
        ]);
        let filtered = NbtSummaryValue::String("filtered".to_string());
        let world = written_book_world(vec![raw], vec![Some(filtered)]);
        let mut input = ClientInputState::default();
        input.set_client_text_filtering_enabled(false);
        let projected = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(projected.runs[0].text, "raw");
        assert!(projected.runs[0].style.bold);
        assert_eq!(projected.interactions[0].source_run_index, 0);
        assert!(projected.interactions[0].clickable);
        assert_eq!(projected.interactions[0].tooltip_runs[0].text, "tip");
        let BookScreenState::View(screen) = world.current_book().unwrap() else {
            panic!("written book must remain a view");
        };
        assert_eq!(
            screen.pages[0].raw.styled()[0].style.font.as_deref(),
            Some("example:book")
        );

        input.set_client_text_filtering_enabled(true);
        let projected = hud_book_screen(&input, &world, None).unwrap();
        assert_eq!(projected.runs[0].text, "filtered");
        assert!(projected.interactions.is_empty());
    }

    #[test]
    fn click_event_revalidates_generation_page_and_source_run() {
        let page = NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::String("next".to_string())),
            entry(
                "click_event",
                NbtSummaryValue::Compound(vec![
                    entry("action", NbtSummaryValue::String("change_page".to_string())),
                    entry("page", NbtSummaryValue::Int(2)),
                ]),
            ),
        ]);
        let world = written_book_world(
            vec![page, NbtSummaryValue::String("two".to_string())],
            vec![None, None],
        );
        let input = ClientInputState::default();
        let screen = hud_book_screen(&input, &world, None).unwrap();
        let target = HudBookHitTarget {
            generation: screen.generation,
            page: 0,
            resource_generation: 0,
            resources_loaded: false,
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
            title: "Guide".to_string(),
            title_filter: None,
            author: "Alex".to_string(),
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
        world
    }
}
