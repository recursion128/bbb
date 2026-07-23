use bbb_protocol::{
    data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    item_types::VANILLA_ITEM_WRITABLE_BOOK_ID,
    packets::{
        InteractionHand, ItemStackSummary as ProtocolItemStackSummary, OpenBook as ProtocolOpenBook,
    },
    RichTextComponent,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

pub const MAX_BOOK_PAGES: usize = 100;
pub const MAX_BOOK_PAGE_UTF16_UNITS: usize = 1024;
pub const MAX_BOOK_TITLE_UTF16_UNITS: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenBookState {
    pub hand: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookViewPageState {
    pub raw: RichTextComponent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filtered: Option<RichTextComponent>,
}

impl BookViewPageState {
    pub fn component_for_filter(&self, should_filter: bool) -> &RichTextComponent {
        if should_filter {
            self.filtered.as_ref().unwrap_or(&self.raw)
        } else {
            &self.raw
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookViewScreenState {
    pub generation: u64,
    pub hand: InteractionHand,
    #[serde(default)]
    pub pages: Vec<BookViewPageState>,
    #[serde(default)]
    pub current_page: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookEditScreenState {
    pub generation: u64,
    pub hand: InteractionHand,
    pub slot: i32,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub current_page: usize,
    #[serde(default)]
    pub sign_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSignScreenState {
    pub generation: u64,
    pub hand: InteractionHand,
    pub slot: i32,
    #[serde(default)]
    pub pages: Vec<String>,
    #[serde(default)]
    pub current_page: usize,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub author: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum BookScreenState {
    View(BookViewScreenState),
    Edit(BookEditScreenState),
    Sign(BookSignScreenState),
}

impl BookScreenState {
    pub fn generation(&self) -> u64 {
        match self {
            Self::View(screen) => screen.generation,
            Self::Edit(screen) => screen.generation,
            Self::Sign(screen) => screen.generation,
        }
    }

    pub fn hand(&self) -> InteractionHand {
        match self {
            Self::View(screen) => screen.hand,
            Self::Edit(screen) => screen.hand,
            Self::Sign(screen) => screen.hand,
        }
    }

    pub fn hand_name(&self) -> &'static str {
        interaction_hand_name(self.hand())
    }

    pub fn current_page(&self) -> usize {
        match self {
            Self::View(screen) => screen.current_page,
            Self::Edit(screen) => screen.current_page,
            Self::Sign(screen) => screen.current_page,
        }
    }

    pub fn page_count(&self) -> usize {
        match self {
            Self::View(screen) => screen.pages.len(),
            Self::Edit(screen) => screen.pages.len(),
            Self::Sign(screen) => screen.pages.len(),
        }
    }

    pub fn is_view(&self) -> bool {
        matches!(self, Self::View(_))
    }

    pub fn is_edit(&self) -> bool {
        matches!(self, Self::Edit(_))
    }

    pub fn is_sign(&self) -> bool {
        matches!(self, Self::Sign(_))
    }

    pub fn origin_slot(&self) -> Option<i32> {
        match self {
            Self::View(_) => None,
            Self::Edit(screen) => Some(screen.slot),
            Self::Sign(screen) => Some(screen.slot),
        }
    }

    pub fn plain_pages_for_filter(&self, should_filter: bool) -> Vec<String> {
        match self {
            Self::View(screen) => screen
                .pages
                .iter()
                .map(|page| page.component_for_filter(should_filter).text().to_string())
                .collect(),
            Self::Edit(screen) => screen.pages.clone(),
            Self::Sign(screen) => screen.pages.clone(),
        }
    }

    pub fn view_page(&self, page: usize) -> Option<&BookViewPageState> {
        let Self::View(screen) = self else {
            return None;
        };
        screen.pages.get(page)
    }

    fn set_current_page(&mut self, page: usize) -> bool {
        match self {
            Self::View(screen) => {
                set_page_index(&mut screen.current_page, screen.pages.len(), page)
            }
            Self::Edit(screen) => {
                set_page_index(&mut screen.current_page, screen.pages.len(), page)
            }
            Self::Sign(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookEditSubmission {
    pub generation: u64,
    pub hand: InteractionHand,
    pub slot: i32,
    pub pages: Vec<String>,
    pub title: Option<String>,
}

impl WorldStore {
    pub fn apply_open_book(&mut self, packet: ProtocolOpenBook) {
        self.counters.open_book_packets += 1;
        self.client_ui.last_open_book = Some(OpenBookState {
            hand: interaction_hand_name(packet.hand).to_string(),
        });
        let Some(item) = self.local_item_in_hand(packet.hand) else {
            return;
        };
        let has_writable_content = self.item_has_writable_book_content(item);
        let Some(pages) = book_view_pages_from_item(item, has_writable_content) else {
            return;
        };
        let generation = self.next_book_screen_generation();
        self.client_ui.replace_screen();
        self.client_ui.current_book = Some(BookScreenState::View(BookViewScreenState {
            generation,
            hand: packet.hand,
            pages,
            current_page: 0,
        }));
    }

    pub fn current_book(&self) -> Option<&BookScreenState> {
        self.client_ui.current_book.as_ref()
    }

    pub fn last_open_book(&self) -> Option<&OpenBookState> {
        self.client_ui.last_open_book.as_ref()
    }

    pub fn open_writable_book_editor(
        &mut self,
        hand: InteractionHand,
        should_filter: bool,
    ) -> bool {
        let Some(item) = self.held_writable_book(hand) else {
            return false;
        };
        let mut pages = filtered_writable_pages(item, should_filter);
        pages.truncate(MAX_BOOK_PAGES);
        if pages.is_empty() {
            pages.push(String::new());
        }
        let Some(slot) = self.book_inventory_slot(hand) else {
            return false;
        };
        let generation = self.next_book_screen_generation();
        self.client_ui.replace_screen();
        self.client_ui.current_book = Some(BookScreenState::Edit(BookEditScreenState {
            generation,
            hand,
            slot,
            pages,
            current_page: 0,
            sign_title: String::new(),
        }));
        true
    }

    pub fn close_current_book(&mut self) -> bool {
        self.client_ui.current_book.take().is_some()
    }

    pub fn set_current_book_page(&mut self, page: usize) -> bool {
        self.client_ui
            .current_book
            .as_mut()
            .is_some_and(|book| book.set_current_page(page))
    }

    pub fn turn_current_book_page(&mut self, delta: i32) -> bool {
        let Some(book) = &mut self.client_ui.current_book else {
            return false;
        };
        if delta > 0 {
            if let BookScreenState::Edit(screen) = book {
                if screen.current_page + 1 == screen.pages.len()
                    && screen.pages.len() < MAX_BOOK_PAGES
                {
                    screen.pages.push(String::new());
                    screen.current_page += 1;
                    return true;
                }
            }
        }
        let current = book.current_page() as i32;
        let max_page = book.page_count().saturating_sub(1) as i32;
        let page = current.saturating_add(delta).clamp(0, max_page);
        book.set_current_page(page as usize)
    }

    pub fn replace_current_book_page(&mut self, generation: u64, value: impl Into<String>) -> bool {
        let value = value.into();
        if value.encode_utf16().count() > MAX_BOOK_PAGE_UTF16_UNITS {
            return false;
        }
        let Some(BookScreenState::Edit(screen)) = &mut self.client_ui.current_book else {
            return false;
        };
        if screen.generation != generation {
            return false;
        }
        let Some(page) = screen.pages.get_mut(screen.current_page) else {
            return false;
        };
        if *page == value {
            return false;
        }
        *page = value;
        true
    }

    pub fn begin_sign_current_book(&mut self, generation: u64) -> bool {
        let Some(current) = self.client_ui.current_book.take() else {
            return false;
        };
        let BookScreenState::Edit(screen) = current else {
            self.client_ui.current_book = Some(current);
            return false;
        };
        if screen.generation != generation {
            self.client_ui.current_book = Some(BookScreenState::Edit(screen));
            return false;
        }
        let author = self.local_player_scoreboard_name().unwrap_or_default();
        self.client_ui.current_book = Some(BookScreenState::Sign(BookSignScreenState {
            generation: screen.generation,
            hand: screen.hand,
            slot: screen.slot,
            pages: screen.pages,
            current_page: screen.current_page,
            title: screen.sign_title,
            author,
        }));
        true
    }

    pub fn cancel_sign_current_book(&mut self, generation: u64) -> bool {
        let Some(current) = self.client_ui.current_book.take() else {
            return false;
        };
        let BookScreenState::Sign(screen) = current else {
            self.client_ui.current_book = Some(current);
            return false;
        };
        if screen.generation != generation {
            self.client_ui.current_book = Some(BookScreenState::Sign(screen));
            return false;
        }
        self.client_ui.current_book = Some(BookScreenState::Edit(BookEditScreenState {
            generation: screen.generation,
            hand: screen.hand,
            slot: screen.slot,
            pages: screen.pages,
            current_page: screen.current_page,
            sign_title: screen.title,
        }));
        true
    }

    pub fn replace_current_book_title(
        &mut self,
        generation: u64,
        value: impl Into<String>,
    ) -> bool {
        let value = value.into();
        if value.encode_utf16().count() > MAX_BOOK_TITLE_UTF16_UNITS {
            return false;
        }
        let Some(BookScreenState::Sign(screen)) = &mut self.client_ui.current_book else {
            return false;
        };
        if screen.generation != generation || screen.title == value {
            return false;
        }
        screen.title = value;
        true
    }

    pub fn current_book_unsigned_submission(&self) -> Option<BookEditSubmission> {
        let BookScreenState::Edit(screen) = self.current_book()? else {
            return None;
        };
        let mut pages = screen.pages.clone();
        while pages.last().is_some_and(String::is_empty) {
            pages.pop();
        }
        self.validated_book_submission(screen.generation, screen.hand, screen.slot, pages, None)
    }

    pub fn current_book_signed_submission(&self) -> Option<BookEditSubmission> {
        let BookScreenState::Sign(screen) = self.current_book()? else {
            return None;
        };
        let title = screen.title.trim();
        if title.is_empty() {
            return None;
        }
        self.validated_book_submission(
            screen.generation,
            screen.hand,
            screen.slot,
            screen.pages.clone(),
            Some(title.to_string()),
        )
    }

    pub fn commit_current_book_submission(&mut self, submission: &BookEditSubmission) -> bool {
        let expected = if submission.title.is_some() {
            self.current_book_signed_submission()
        } else {
            self.current_book_unsigned_submission()
        };
        if expected.as_ref() != Some(submission) {
            return false;
        }
        if submission.title.is_none()
            && !self
                .inventory
                .predict_writable_book_pages(submission.slot, submission.pages.clone())
        {
            return false;
        }
        self.client_ui.current_book = None;
        true
    }

    pub(crate) fn close_invalid_writable_book_screen(&mut self) -> bool {
        let Some(current) = self.current_book() else {
            return false;
        };
        if current.is_view()
            || (current.origin_slot() == self.book_inventory_slot(current.hand())
                && self.held_writable_book(current.hand()).is_some())
        {
            return false;
        }
        self.client_ui.current_book = None;
        true
    }

    fn validated_book_submission(
        &self,
        generation: u64,
        hand: InteractionHand,
        slot: i32,
        pages: Vec<String>,
        title: Option<String>,
    ) -> Option<BookEditSubmission> {
        if pages.len() > MAX_BOOK_PAGES
            || pages
                .iter()
                .any(|page| page.encode_utf16().count() > MAX_BOOK_PAGE_UTF16_UNITS)
            || title
                .as_ref()
                .is_some_and(|title| title.encode_utf16().count() > MAX_BOOK_TITLE_UTF16_UNITS)
        {
            return None;
        }
        if self.book_inventory_slot(hand) != Some(slot) {
            return None;
        }
        self.held_writable_book(hand)?;
        Some(BookEditSubmission {
            generation,
            hand,
            slot,
            pages,
            title,
        })
    }

    fn held_writable_book(&self, hand: InteractionHand) -> Option<&ProtocolItemStackSummary> {
        let item = self.local_item_in_hand(hand)?;
        (item.item_id == Some(VANILLA_ITEM_WRITABLE_BOOK_ID)
            && self.item_has_writable_book_content(item))
        .then_some(item)
    }

    fn item_has_writable_book_content(&self, item: &ProtocolItemStackSummary) -> bool {
        !item
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)
            && (item.item_id == Some(VANILLA_ITEM_WRITABLE_BOOK_ID)
                || self.item_stack_has_effective_component(
                    item,
                    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
                ))
    }

    fn book_inventory_slot(&self, hand: InteractionHand) -> Option<i32> {
        match hand {
            InteractionHand::MainHand => {
                let slot = self.local_player.selected_hotbar_slot;
                (slot <= 8).then_some(i32::from(slot))
            }
            InteractionHand::OffHand => Some(40),
        }
    }

    fn next_book_screen_generation(&mut self) -> u64 {
        self.client_ui.book_screen_generation =
            self.client_ui.book_screen_generation.saturating_add(1);
        self.client_ui.book_screen_generation
    }
}

fn set_page_index(current_page: &mut usize, page_count: usize, page: usize) -> bool {
    let page = page.min(page_count.saturating_sub(1));
    if page == *current_page {
        return false;
    }
    *current_page = page;
    true
}

fn interaction_hand_name(hand: InteractionHand) -> &'static str {
    match hand {
        InteractionHand::MainHand => "main_hand",
        InteractionHand::OffHand => "off_hand",
    }
}

fn book_view_pages_from_item(
    item: &ProtocolItemStackSummary,
    has_writable_content: bool,
) -> Option<Vec<BookViewPageState>> {
    if let Some(written) = &item.component_patch.written_book {
        return Some(
            written
                .pages
                .iter()
                .enumerate()
                .map(|(index, plain)| BookViewPageState {
                    raw: exact_or_plain_component(
                        item.component_patch
                            .written_book_pages_nbt
                            .get(index)
                            .and_then(Option::as_ref),
                        plain,
                    ),
                    filtered: written.page_filters.get(index).and_then(|filtered| {
                        filtered.as_ref().map(|plain| {
                            exact_or_plain_component(
                                item.component_patch
                                    .written_book_page_filters_nbt
                                    .get(index)
                                    .and_then(Option::as_ref),
                                plain,
                            )
                        })
                    }),
                })
                .collect(),
        );
    }
    if has_writable_content {
        return Some(
            item.component_patch
                .writable_book_pages
                .iter()
                .enumerate()
                .map(|(index, raw)| BookViewPageState {
                    raw: RichTextComponent::plain(raw.clone()),
                    filtered: item
                        .component_patch
                        .writable_book_page_filters
                        .get(index)
                        .and_then(|filtered| filtered.as_ref())
                        .map(|filtered| RichTextComponent::plain(filtered.clone())),
                })
                .collect(),
        );
    }
    None
}

fn exact_or_plain_component(
    exact: Option<&bbb_protocol::NbtSummaryValue>,
    plain: &str,
) -> RichTextComponent {
    exact
        .cloned()
        .and_then(|component| RichTextComponent::try_from_component_nbt(component).ok())
        .unwrap_or_else(|| RichTextComponent::plain(plain))
}

fn filtered_writable_pages(item: &ProtocolItemStackSummary, should_filter: bool) -> Vec<String> {
    item.component_patch
        .writable_book_pages
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            if should_filter {
                item.component_patch
                    .writable_book_page_filters
                    .get(index)
                    .and_then(|filtered| filtered.as_ref())
                    .unwrap_or(raw)
                    .clone()
            } else {
                raw.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        data_component_types::VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        item_types::VANILLA_ITEM_WRITABLE_BOOK_ID,
        packets::{DataComponentPatchSummary, SetPlayerInventory, WrittenBookContentSummary},
        ComponentClickEvent, NbtSummaryEntry, NbtSummaryValue,
    };

    use super::*;

    #[test]
    fn book_screen_view_preserves_exact_raw_and_filtered_components() {
        let raw = styled_page("Exact raw", "red", 2);
        let filtered = styled_page("Filtered", "blue", 1);
        let mut item = item_stack(1200);
        item.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".to_string(),
            title_filter: None,
            author: "Alex".to_string(),
            generation: 0,
            pages: vec!["Exact raw".to_string()],
            page_filters: vec![Some("Filtered".to_string())],
            resolved: true,
        });
        item.component_patch.written_book_pages_nbt = vec![Some(raw.clone())];
        item.component_patch.written_book_page_filters_nbt = vec![Some(filtered.clone())];
        item.component_patch.writable_book_pages = vec!["Writable loses".to_string()];
        item.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);

        let mut world = WorldStore::new();
        set_item(&mut world, 0, item);
        world.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::MainHand,
        });

        let BookScreenState::View(screen) = world.current_book().unwrap() else {
            panic!("written content must open a view");
        };
        assert_eq!(screen.generation, 1);
        assert_eq!(screen.pages.len(), 1);
        assert_eq!(screen.pages[0].raw.component(), &raw);
        assert_eq!(
            screen.pages[0].filtered.as_ref().unwrap().component(),
            &filtered
        );
        assert_eq!(
            screen.pages[0].raw.styled()[0].style.click_event,
            Some(ComponentClickEvent::ChangePage { page: 2 })
        );
        assert_eq!(
            world.current_book().unwrap().plain_pages_for_filter(false),
            vec!["Exact raw".to_string()]
        );
        assert_eq!(
            world.current_book().unwrap().plain_pages_for_filter(true),
            vec!["Filtered".to_string()]
        );
    }

    #[test]
    fn book_screen_editor_selects_filtered_pages_and_retains_sign_title_on_cancel() {
        let mut world = WorldStore::new();
        let mut item = writable_book(vec!["raw", "second"]);
        item.component_patch.writable_book_page_filters = vec![Some("filtered".to_string()), None];
        set_item(&mut world, 40, item);

        assert!(world.open_writable_book_editor(InteractionHand::OffHand, true));
        let generation = world.current_book().unwrap().generation();
        let BookScreenState::Edit(screen) = world.current_book().unwrap() else {
            panic!("writable use must open the editor");
        };
        assert_eq!(
            screen.pages,
            vec!["filtered".to_string(), "second".to_string()]
        );
        assert_eq!(screen.current_page, 0);
        assert!(world.begin_sign_current_book(generation));
        assert!(world.replace_current_book_title(generation, "  Guide  "));
        assert!(world.cancel_sign_current_book(generation));
        assert!(world.begin_sign_current_book(generation));
        let BookScreenState::Sign(screen) = world.current_book().unwrap() else {
            panic!("sign must reopen");
        };
        assert_eq!(screen.title, "  Guide  ");
        assert_eq!(screen.pages, vec!["filtered", "second"]);
    }

    #[test]
    fn book_screen_editor_initializes_empty_page_and_enforces_bounds() {
        let mut world = WorldStore::new();
        set_item(&mut world, 0, item_stack(VANILLA_ITEM_WRITABLE_BOOK_ID));
        assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
        let generation = world.current_book().unwrap().generation();
        let BookScreenState::Edit(screen) = world.current_book().unwrap() else {
            panic!("writable use must open the editor");
        };
        assert_eq!(screen.pages, vec![""]);
        assert!(!world.replace_current_book_page(generation, "x".repeat(1025)));
        assert!(world.replace_current_book_page(generation, "x".repeat(1024)));
        assert!(world.turn_current_book_page(1));
        let BookScreenState::Edit(screen) = world.current_book().unwrap() else {
            panic!("editor must stay open");
        };
        assert_eq!(screen.pages.len(), 2);
        assert_eq!(screen.current_page, 1);
        assert!(!world.replace_current_book_title(generation, "not signing"));
        assert!(world.begin_sign_current_book(generation));
        assert!(!world.replace_current_book_title(generation, "x".repeat(16)));
        assert!(world.replace_current_book_title(generation, "x".repeat(15)));
    }

    #[test]
    fn book_screen_unsigned_submission_trims_pages_and_predicts_writable_content() {
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(3));
        set_item(&mut world, 3, writable_book(vec!["old"]));
        assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
        let generation = world.current_book().unwrap().generation();
        assert!(world.replace_current_book_page(generation, "saved"));
        assert!(world.turn_current_book_page(1));

        let submission = world.current_book_unsigned_submission().unwrap();
        assert_eq!(submission.slot, 3);
        assert_eq!(submission.pages, vec!["saved"]);
        assert_eq!(submission.title, None);
        assert!(world.commit_current_book_submission(&submission));
        assert!(world.current_book().is_none());
        let held = world.local_item_in_hand(InteractionHand::MainHand).unwrap();
        assert_eq!(
            held.component_patch.writable_book_pages,
            vec!["saved".to_string()]
        );
        assert_eq!(held.component_patch.writable_book_page_filters, vec![None]);

        set_item(&mut world, 3, writable_book(vec!["server correction"]));
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .component_patch
                .writable_book_pages,
            vec!["server correction".to_string()]
        );
    }

    #[test]
    fn book_screen_signed_submission_keeps_trailing_pages_and_waits_for_server_item() {
        let mut world = WorldStore::new();
        set_item(&mut world, 40, writable_book(vec!["signed", ""]));
        let before = world
            .local_item_in_hand(InteractionHand::OffHand)
            .unwrap()
            .clone();
        assert!(world.open_writable_book_editor(InteractionHand::OffHand, false));
        let generation = world.current_book().unwrap().generation();
        assert!(world.begin_sign_current_book(generation));
        assert_eq!(world.current_book_signed_submission(), None);
        assert!(world.replace_current_book_title(generation, "  Title  "));

        let submission = world.current_book_signed_submission().unwrap();
        assert_eq!(submission.slot, 40);
        assert_eq!(submission.pages, vec!["signed", ""]);
        assert_eq!(submission.title.as_deref(), Some("Title"));
        assert!(world.commit_current_book_submission(&submission));
        assert_eq!(
            world.local_item_in_hand(InteractionHand::OffHand),
            Some(&before)
        );
    }

    #[test]
    fn book_screen_submission_revalidates_generation_slot_and_writable_component() {
        let mut world = WorldStore::new();
        set_item(&mut world, 0, writable_book(vec!["draft"]));
        set_item(&mut world, 1, writable_book(vec!["other"]));
        assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
        let stale_generation = world.current_book().unwrap().generation();
        let stale_submission = world.current_book_unsigned_submission().unwrap();

        assert!(world.set_local_selected_hotbar_slot(1));
        assert!(world.current_book().is_none());
        assert!(!world.commit_current_book_submission(&stale_submission));
        assert!(world.open_writable_book_editor(InteractionHand::MainHand, false));
        assert_ne!(world.current_book().unwrap().generation(), stale_generation);
        assert!(!world.replace_current_book_page(stale_generation, "stale"));

        set_item(&mut world, 1, item_stack(42));
        assert!(world.current_book().is_none());
        let mut removed = writable_book(vec!["removed"]);
        removed.component_patch.removed_type_ids =
            vec![VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID];
        set_item(&mut world, 1, removed);
        assert!(!world.open_writable_book_editor(InteractionHand::MainHand, false));
    }

    #[test]
    fn book_screen_unrelated_inventory_update_preserves_editor_but_held_loss_closes_it() {
        let mut world = WorldStore::new();
        set_item(&mut world, 40, writable_book(vec!["draft"]));
        assert!(world.open_writable_book_editor(InteractionHand::OffHand, false));
        set_item(&mut world, 2, item_stack(42));
        assert!(world.current_book().is_some());
        set_item(&mut world, 40, ProtocolItemStackSummary::empty());
        assert!(world.current_book().is_none());
    }

    fn item_stack(item_id: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn writable_book(pages: Vec<&str>) -> ProtocolItemStackSummary {
        let mut item = item_stack(VANILLA_ITEM_WRITABLE_BOOK_ID);
        item.component_patch.added = 1;
        item.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
        item.component_patch.writable_book_pages = pages.into_iter().map(str::to_string).collect();
        item.component_patch.writable_book_page_filters =
            vec![None; item.component_patch.writable_book_pages.len()];
        item
    }

    fn set_item(world: &mut WorldStore, slot: i32, item: ProtocolItemStackSummary) {
        assert!(world.apply_set_player_inventory(SetPlayerInventory { slot, item }));
    }

    fn styled_page(text: &str, color: &str, page: i32) -> NbtSummaryValue {
        NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String(text.to_string()),
            },
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".to_string(),
                value: NbtSummaryValue::String(color.to_string()),
            },
            NbtSummaryEntry {
                name: "click_event".to_string(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".to_string(),
                        value: NbtSummaryValue::String("change_page".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "page".to_string(),
                        value: NbtSummaryValue::Int(page),
                    },
                ]),
            },
        ])
    }
}
