use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
        VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
    },
    item_types::VANILLA_ITEM_WRITABLE_BOOK_ID,
    menu_types::VANILLA_MENU_TYPE_LECTERN_ID,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LecternBookViewState {
    pub container_id: i32,
    pub revision: u64,
    pub pages: Vec<BookViewPageState>,
    pub current_page: usize,
    pub may_build: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "Vec<u16>", into = "Vec<u16>")]
pub struct BookEditorText {
    units: Vec<u16>,
    display: String,
}

impl BookEditorText {
    pub fn units(&self) -> &[u16] {
        &self.units
    }

    pub fn display(&self) -> &str {
        &self.display
    }
}

impl From<Vec<u16>> for BookEditorText {
    fn from(units: Vec<u16>) -> Self {
        Self {
            display: String::from_utf16_lossy(&units),
            units,
        }
    }
}

impl From<String> for BookEditorText {
    fn from(value: String) -> Self {
        Self::from(value.encode_utf16().collect::<Vec<_>>())
    }
}

impl From<&str> for BookEditorText {
    fn from(value: &str) -> Self {
        Self::from(value.encode_utf16().collect::<Vec<_>>())
    }
}

impl From<bbb_protocol::JavaString> for BookEditorText {
    fn from(value: bbb_protocol::JavaString) -> Self {
        Self::from(Vec::<u16>::from(value))
    }
}

impl From<BookEditorText> for Vec<u16> {
    fn from(value: BookEditorText) -> Self {
        value.units
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookEditScreenState {
    pub generation: u64,
    pub hand: InteractionHand,
    pub slot: i32,
    #[serde(default)]
    pub pages: Vec<BookEditorText>,
    #[serde(default)]
    pub current_page: usize,
    #[serde(default)]
    pub sign_title: BookEditorText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookSignScreenState {
    pub generation: u64,
    pub hand: InteractionHand,
    pub slot: i32,
    #[serde(default)]
    pub pages: Vec<BookEditorText>,
    #[serde(default)]
    pub current_page: usize,
    #[serde(default)]
    pub title: BookEditorText,
    #[serde(default)]
    pub author: String,
}

impl BookSignScreenState {
    // BookSignScreen.java:43-55; StringUtil.java:88-94.
    pub fn can_finalize_with_button(&self) -> bool {
        !bbb_protocol::JavaString::minecraft_units_are_blank(self.title.units())
    }
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
            Self::Edit(screen) => screen
                .pages
                .iter()
                .map(|page| page.display().to_string())
                .collect(),
            Self::Sign(screen) => screen
                .pages
                .iter()
                .map(|page| page.display().to_string())
                .collect(),
        }
    }

    pub fn view_page(&self, page: usize) -> Option<&BookViewPageState> {
        let Self::View(screen) = self else {
            return None;
        };
        screen.pages.get(page)
    }

    pub fn set_current_page(&mut self, page: usize) -> bool {
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

    /// `pageBack` / `pageForward`; the editor appends a page past its last.
    pub fn turn_page(&mut self, delta: i32) -> bool {
        if let Self::Edit(screen) = self {
            if delta > 0
                && screen.current_page + 1 == screen.pages.len()
                && screen.pages.len() < MAX_BOOK_PAGES
            {
                screen.pages.push(BookEditorText::default());
                screen.current_page += 1;
                return true;
            }
        }
        let current = self.current_page() as i32;
        let max_page = self.page_count().saturating_sub(1) as i32;
        let page = current.saturating_add(delta).clamp(0, max_page);
        self.set_current_page(page as usize)
    }

    pub fn replace_page(&mut self, value: impl Into<BookEditorText>) -> bool {
        let value = value.into();
        if value.units().len() > MAX_BOOK_PAGE_UTF16_UNITS {
            return false;
        }
        let Self::Edit(screen) = self else {
            return false;
        };
        let Some(page) = screen.pages.get_mut(screen.current_page) else {
            return false;
        };
        if *page == value {
            return false;
        }
        *page = value;
        true
    }

    /// The editor's sign button opens the sign screen over the same pages.
    pub fn begin_sign(&mut self, author: String) -> bool {
        let Self::Edit(screen) = self else {
            return false;
        };
        *self = Self::Sign(BookSignScreenState {
            generation: screen.generation,
            hand: screen.hand,
            slot: screen.slot,
            pages: std::mem::take(&mut screen.pages),
            current_page: screen.current_page,
            title: std::mem::take(&mut screen.sign_title),
            author,
        });
        true
    }

    /// The sign screen's cancel returns to the editor, keeping the title.
    pub fn cancel_sign(&mut self) -> bool {
        let Self::Sign(screen) = self else {
            return false;
        };
        *self = Self::Edit(BookEditScreenState {
            generation: screen.generation,
            hand: screen.hand,
            slot: screen.slot,
            pages: std::mem::take(&mut screen.pages),
            current_page: screen.current_page,
            sign_title: std::mem::take(&mut screen.title),
        });
        true
    }

    pub fn replace_title(&mut self, value: impl Into<BookEditorText>) -> bool {
        let value = value.into();
        if value.units().len() > MAX_BOOK_TITLE_UTF16_UNITS {
            return false;
        }
        let Self::Sign(screen) = self else {
            return false;
        };
        if screen.title == value {
            return false;
        }
        screen.title = value;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookEditSubmission {
    pub hand: InteractionHand,
    pub slot: i32,
    pub pages: Vec<Vec<u16>>,
    pub title: Option<Vec<u16>>,
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
        let Some(pages) = self.book_view_pages_for_item(item) else {
            return;
        };
        let generation = self.next_book_screen_generation();
        self.client_ui.replace_screen();
        self.push_screen_request(crate::ScreenRequest::Book(BookScreenState::View(
            BookViewScreenState {
                generation,
                hand: packet.hand,
                pages,
                current_page: 0,
            },
        )));
    }

    pub fn lectern_book_view(&self) -> Option<LecternBookViewState> {
        let container = self
            .inventory
            .open_container
            .as_ref()
            .filter(|container| container.menu_type_id == Some(VANILLA_MENU_TYPE_LECTERN_ID))?;
        let pages = container
            .slots
            .iter()
            .find(|slot| slot.slot == 0)
            .and_then(|slot| self.book_view_pages_for_item(&slot.item))
            .unwrap_or_default();
        let current_page = container
            .data_values
            .iter()
            .find_map(|value| (value.id == 0).then_some(i32::from(value.value)))
            .unwrap_or_default()
            .clamp(0, pages.len().saturating_sub(1) as i32) as usize;
        Some(LecternBookViewState {
            container_id: container.container_id,
            revision: self.inventory.open_container_revision,
            pages,
            current_page,
            may_build: !self.local_player_is_adventure() && !self.local_player_is_spectator(),
        })
    }

    pub fn last_open_book(&self) -> Option<&OpenBookState> {
        self.client_ui.last_open_book.as_ref()
    }

    /// `BookEditScreen` over the writable book in `hand`.
    pub fn writable_book_editor(
        &mut self,
        hand: InteractionHand,
        should_filter: bool,
    ) -> Option<BookScreenState> {
        let mut pages = filtered_writable_pages(self.held_writable_book(hand)?, should_filter);
        pages.truncate(MAX_BOOK_PAGES);
        if pages.is_empty() {
            pages.push(Default::default());
        }
        let slot = self.book_inventory_slot(hand)?;
        Some(BookScreenState::Edit(BookEditScreenState {
            generation: self.next_book_screen_generation(),
            hand,
            slot,
            pages: pages.into_iter().map(BookEditorText::from).collect(),
            current_page: 0,
            sign_title: BookEditorText::default(),
        }))
    }

    /// The `ServerboundEditBookPacket` the editor's Done or the sign screen's
    /// finalize would send now.
    pub fn book_submission(&self, book: &BookScreenState) -> Option<BookEditSubmission> {
        match book {
            BookScreenState::View(_) => None,
            BookScreenState::Edit(screen) => {
                let mut pages = screen
                    .pages
                    .iter()
                    .map(|page| page.units().to_vec())
                    .collect::<Vec<_>>();
                while pages.last().is_some_and(Vec::is_empty) {
                    pages.pop();
                }
                self.validated_book_submission(screen.hand, screen.slot, pages, None)
            }
            BookScreenState::Sign(screen) => {
                let title = screen.title.units();
                // BookSignScreen.java:80-84: confirmation checks before Java trim.
                if title.is_empty() {
                    return None;
                }
                let start = title
                    .iter()
                    .position(|unit| *unit > 0x20)
                    .unwrap_or(title.len());
                let end = title
                    .iter()
                    .rposition(|unit| *unit > 0x20)
                    .map_or(start, |index| index + 1);
                let title = &title[start..end];
                self.validated_book_submission(
                    screen.hand,
                    screen.slot,
                    screen
                        .pages
                        .iter()
                        .map(|page| page.units().to_vec())
                        .collect(),
                    Some(title.to_vec()),
                )
            }
        }
    }

    /// `BookEditScreen.updateLocalCopy` before the unsigned packet; a signed
    /// book waits for the server's item.
    pub fn commit_book_submission(&mut self, submission: &BookEditSubmission) -> bool {
        submission.title.is_some()
            || self.inventory.predict_writable_book_pages(
                submission.slot,
                submission.pages.iter().cloned().map(Into::into).collect(),
            )
    }

    /// Whether the writable book an edit or sign screen writes is still in
    /// the slot it was opened from.
    pub fn book_is_held(&self, book: &BookScreenState) -> bool {
        book.is_view()
            || (book.origin_slot() == self.book_inventory_slot(book.hand())
                && self.held_writable_book(book.hand()).is_some())
    }

    fn validated_book_submission(
        &self,
        hand: InteractionHand,
        slot: i32,
        pages: Vec<Vec<u16>>,
        title: Option<Vec<u16>>,
    ) -> Option<BookEditSubmission> {
        if pages.len() > MAX_BOOK_PAGES
            || pages
                .iter()
                .any(|page| page.len() > MAX_BOOK_PAGE_UTF16_UNITS)
            || title
                .as_ref()
                .is_some_and(|title| title.len() > MAX_BOOK_TITLE_UTF16_UNITS)
        {
            return None;
        }
        if self.book_inventory_slot(hand) != Some(slot) {
            return None;
        }
        self.held_writable_book(hand)?;
        Some(BookEditSubmission {
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

    fn item_has_written_book_content(&self, item: &ProtocolItemStackSummary) -> bool {
        !item
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)
            && (item.component_patch.written_book.is_some()
                || self.item_stack_has_effective_component(
                    item,
                    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
                ))
    }

    fn book_view_pages_for_item(
        &self,
        item: &ProtocolItemStackSummary,
    ) -> Option<Vec<BookViewPageState>> {
        book_view_pages_from_item(
            item,
            self.item_has_written_book_content(item),
            self.item_has_writable_book_content(item),
        )
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
    has_written_content: bool,
    has_writable_content: bool,
) -> Option<Vec<BookViewPageState>> {
    if has_written_content {
        let written = item.component_patch.written_book.as_ref()?;
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
                    raw: RichTextComponent::try_from_component_nbt(raw.to_nbt())
                        .expect("book string is a literal component"),
                    filtered: item
                        .component_patch
                        .writable_book_page_filters
                        .get(index)
                        .and_then(|filtered| filtered.as_ref())
                        .map(|filtered| {
                            RichTextComponent::try_from_component_nbt(filtered.to_nbt())
                                .expect("book string is a literal component")
                        }),
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

fn filtered_writable_pages(
    item: &ProtocolItemStackSummary,
    should_filter: bool,
) -> Vec<bbb_protocol::JavaString> {
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
        menu_types::VANILLA_MENU_TYPE_LECTERN_ID,
        packets::{
            ContainerSetContent, ContainerSetData, DataComponentPatchSummary, OpenScreen,
            SetPlayerInventory, WrittenBookContentSummary,
        },
        ComponentClickEvent, NbtSummaryEntry, NbtSummaryValue,
    };

    use super::*;

    #[test]
    fn book_screen_view_preserves_exact_raw_and_filtered_components() {
        let raw = styled_page("Exact raw", "red", 2);
        let filtered = styled_page("Filtered", "blue", 1);
        let mut item = item_stack(1200);
        item.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: None,
            author: "Alex".into(),
            generation: 0,
            pages: vec!["Exact raw".to_string()],
            page_filters: vec![Some("Filtered".to_string())],
            resolved: true,
        });
        item.component_patch.written_book_pages_nbt = vec![Some(raw.clone())];
        item.component_patch.written_book_page_filters_nbt = vec![Some(filtered.clone())];
        item.component_patch.writable_book_pages = vec!["Writable loses".into()];
        item.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);

        let mut world = WorldStore::new();
        set_item(&mut world, 0, item);
        world.apply_open_book(ProtocolOpenBook {
            hand: InteractionHand::MainHand,
        });

        let book = opened_book(&mut world);
        let BookScreenState::View(screen) = &book else {
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
            book.plain_pages_for_filter(false),
            vec!["Exact raw".to_string()]
        );
        assert_eq!(
            book.plain_pages_for_filter(true),
            vec!["Filtered".to_string()]
        );
    }

    #[test]
    fn lectern_book_view_reuses_exact_filtered_page_access_and_server_page() {
        let raw = styled_page("Exact raw", "red", 2);
        let filtered = styled_page("Filtered", "blue", 1);
        let mut item = item_stack(1200);
        item.component_patch.written_book = Some(WrittenBookContentSummary {
            title: "Guide".into(),
            title_filter: None,
            author: "Alex".into(),
            generation: 0,
            pages: vec!["Exact raw".to_string(), "Second".to_string()],
            page_filters: vec![Some("Filtered".to_string()), None],
            resolved: true,
        });
        item.component_patch.written_book_pages_nbt =
            vec![Some(raw.clone()), Some(NbtSummaryValue::string("Second"))];
        item.component_patch.written_book_page_filters_nbt = vec![Some(filtered.clone()), None];

        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
            title: RichTextComponent::plain("Lectern"),
        });
        assert!(world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 2,
            items: vec![item],
            carried_item: ProtocolItemStackSummary::empty(),
        }));
        assert!(world.apply_container_set_data(ContainerSetData {
            container_id: 7,
            id: 0,
            value: 99,
        }));

        let lectern = world.lectern_book_view().unwrap();
        assert_eq!(lectern.container_id, 7);
        assert_eq!(lectern.revision, 3);
        assert_eq!(lectern.current_page, 1);
        assert!(lectern.may_build);
        assert_eq!(lectern.pages[0].raw.component(), &raw);
        assert_eq!(
            lectern.pages[0].filtered.as_ref().unwrap().component(),
            &filtered
        );
    }

    #[test]
    fn lectern_book_view_uses_empty_access_and_writable_literal_filters() {
        let mut world = WorldStore::new();
        world.apply_open_screen(OpenScreen {
            container_id: 7,
            menu_type_id: VANILLA_MENU_TYPE_LECTERN_ID,
            title: RichTextComponent::plain("Lectern"),
        });
        let empty = world.lectern_book_view().unwrap();
        assert!(empty.pages.is_empty());
        assert_eq!(empty.current_page, 0);

        let mut item = item_stack(VANILLA_ITEM_WRITABLE_BOOK_ID);
        item.component_patch.writable_book_pages = vec!["raw".into(), "second".into()];
        item.component_patch.writable_book_page_filters = vec![Some("filtered".into()), None];
        assert!(world.apply_container_set_content(ContainerSetContent {
            container_id: 7,
            state_id: 2,
            items: vec![item],
            carried_item: ProtocolItemStackSummary::empty(),
        }));

        let lectern = world.lectern_book_view().unwrap();
        assert_eq!(lectern.pages[0].raw.text(), "raw");
        assert_eq!(
            lectern.pages[0].filtered.as_ref().unwrap().text(),
            "filtered"
        );
        assert_eq!(lectern.pages[1].raw.text(), "second");
        assert!(lectern.pages[1].filtered.is_none());
    }

    #[test]
    fn book_screen_raw_utf16_survives_sign_cancel_and_state_roundtrip() {
        let mut world = WorldStore::new();
        set_item(&mut world, 0, writable_book(vec![""]));
        let mut book = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();
        let page = vec![0xd800, 65, 0xdc00];
        assert!(book.replace_page(page.clone()));
        assert!(book.begin_sign(String::new()));
        assert!(book.replace_title(vec![0xd800]));
        assert!(book.cancel_sign());
        let saved = serde_json::to_string(&book).unwrap();
        let restored: BookScreenState = serde_json::from_str(&saved).unwrap();
        let BookScreenState::Edit(screen) = restored else {
            panic!("edit state");
        };
        assert_eq!(screen.pages[0].units(), page);
        assert_eq!(screen.pages[0].display(), "�A�");
        assert_eq!(screen.sign_title.units(), &[0xd800]);
    }

    #[test]
    fn book_screen_editor_selects_filtered_pages_and_retains_sign_title_on_cancel() {
        let mut world = WorldStore::new();
        let mut item = writable_book(vec!["raw", "second"]);
        item.component_patch.writable_book_page_filters = vec![Some("filtered".into()), None];
        set_item(&mut world, 40, item);

        let mut book = world
            .writable_book_editor(InteractionHand::OffHand, true)
            .unwrap();
        let BookScreenState::Edit(screen) = &book else {
            panic!("writable use must open the editor");
        };
        assert_eq!(
            screen
                .pages
                .iter()
                .map(BookEditorText::display)
                .collect::<Vec<_>>(),
            vec!["filtered", "second"]
        );
        assert_eq!(screen.current_page, 0);
        assert!(book.begin_sign(String::new()));
        assert!(book.replace_title("  Guide  "));
        assert!(book.cancel_sign());
        assert!(book.begin_sign(String::new()));
        let BookScreenState::Sign(screen) = &book else {
            panic!("sign must reopen");
        };
        assert_eq!(screen.title.display(), "  Guide  ");
        assert_eq!(
            screen
                .pages
                .iter()
                .map(BookEditorText::display)
                .collect::<Vec<_>>(),
            vec!["filtered", "second"]
        );
    }

    #[test]
    fn book_screen_editor_initializes_empty_page_and_enforces_bounds() {
        let mut world = WorldStore::new();
        set_item(&mut world, 0, item_stack(VANILLA_ITEM_WRITABLE_BOOK_ID));
        let mut book = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();
        let BookScreenState::Edit(screen) = &book else {
            panic!("writable use must open the editor");
        };
        assert_eq!(
            screen
                .pages
                .iter()
                .map(BookEditorText::display)
                .collect::<Vec<_>>(),
            vec![""]
        );
        assert!(!book.replace_page("x".repeat(1025)));
        assert!(book.replace_page("x".repeat(1024)));
        assert!(book.turn_page(1));
        let BookScreenState::Edit(screen) = &book else {
            panic!("editor must stay open");
        };
        assert_eq!(screen.pages.len(), 2);
        assert_eq!(screen.current_page, 1);
        assert!(!book.replace_title("not signing"));
        assert!(book.begin_sign(String::new()));
        assert!(!book.replace_title("x".repeat(16)));
        assert!(book.replace_title("x".repeat(15)));
    }

    #[test]
    fn book_screen_submission_preserves_raw_units_and_java_title_trim() {
        let mut world = WorldStore::new();
        set_item(&mut world, 0, writable_book(vec!["old"]));
        let mut book = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();
        let page = vec![0xd800, 0x20ac, 0xdc00];
        assert!(book.replace_page(page.clone()));
        assert_eq!(
            world.book_submission(&book).unwrap().pages,
            vec![page.clone()]
        );
        assert!(book.begin_sign(String::new()));
        assert!(book.replace_title(vec![32, 0xa0, 0xd800, 32]));
        let submission = world.book_submission(&book).unwrap();
        assert_eq!(submission.pages, vec![page]);
        assert_eq!(submission.title, Some(vec![0xa0, 0xd800]));
        assert!(world.commit_book_submission(&submission));
    }

    #[test]
    fn book_screen_unsigned_submission_trims_pages_and_predicts_writable_content() {
        let mut world = WorldStore::new();
        assert!(world.set_local_selected_hotbar_slot(3));
        set_item(&mut world, 3, writable_book(vec!["old"]));
        let mut book = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();
        assert!(book.replace_page("saved"));
        assert!(book.turn_page(1));

        let submission = world.book_submission(&book).unwrap();
        assert_eq!(submission.slot, 3);
        assert_eq!(
            submission.pages,
            vec!["saved".encode_utf16().collect::<Vec<_>>()]
        );
        assert_eq!(submission.title, None);
        assert!(world.commit_book_submission(&submission));
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
        let mut book = world
            .writable_book_editor(InteractionHand::OffHand, false)
            .unwrap();
        assert!(book.begin_sign(String::new()));
        assert_eq!(world.book_submission(&book), None);
        assert!(book.replace_title("  Title  "));

        let submission = world.book_submission(&book).unwrap();
        assert_eq!(submission.slot, 40);
        assert_eq!(
            submission.pages,
            vec!["signed".encode_utf16().collect::<Vec<_>>(), vec![]]
        );
        assert_eq!(submission.title, Some("Title".encode_utf16().collect()));
        assert!(world.commit_book_submission(&submission));
        assert_eq!(
            world.local_item_in_hand(InteractionHand::OffHand),
            Some(&before)
        );
    }

    #[test]
    fn book_screen_submission_revalidates_slot_and_writable_component() {
        let mut world = WorldStore::new();
        set_item(&mut world, 0, writable_book(vec!["draft"]));
        set_item(&mut world, 1, writable_book(vec!["other"]));
        let stale = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();

        assert!(world.set_local_selected_hotbar_slot(1));
        assert!(!world.book_is_held(&stale));
        assert_eq!(world.book_submission(&stale), None);
        let book = world
            .writable_book_editor(InteractionHand::MainHand, false)
            .unwrap();
        assert_ne!(book.generation(), stale.generation());

        set_item(&mut world, 1, item_stack(42));
        assert!(!world.book_is_held(&book));
        let mut removed = writable_book(vec!["removed"]);
        removed.component_patch.removed_type_ids =
            vec![VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID];
        set_item(&mut world, 1, removed);
        assert_eq!(
            world.writable_book_editor(InteractionHand::MainHand, false),
            None
        );
    }

    #[test]
    fn book_screen_unrelated_inventory_update_keeps_editor_but_held_loss_does_not() {
        let mut world = WorldStore::new();
        set_item(&mut world, 40, writable_book(vec!["draft"]));
        let book = world
            .writable_book_editor(InteractionHand::OffHand, false)
            .unwrap();
        set_item(&mut world, 2, item_stack(42));
        assert!(world.book_is_held(&book));
        set_item(&mut world, 40, ProtocolItemStackSummary::empty());
        assert!(!world.book_is_held(&book));
    }

    fn opened_book(world: &mut WorldStore) -> BookScreenState {
        world
            .take_screen_requests()
            .into_iter()
            .find_map(|request| match request {
                crate::ScreenRequest::Book(book) => Some(book),
                _ => None,
            })
            .expect("the packet opens a book screen")
    }

    fn item_stack(item_id: i32) -> ProtocolItemStackSummary {
        ProtocolItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        }
    }

    fn writable_book(pages: Vec<&str>) -> ProtocolItemStackSummary {
        let mut item = item_stack(VANILLA_ITEM_WRITABLE_BOOK_ID);
        item.component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID);
        item.component_patch.writable_book_pages = pages.into_iter().map(Into::into).collect();
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
                name: "text".into(),
                value: NbtSummaryValue::string(text),
            },
            NbtSummaryEntry {
                name: "bold".into(),
                value: NbtSummaryValue::Byte(1),
            },
            NbtSummaryEntry {
                name: "color".into(),
                value: NbtSummaryValue::string(color),
            },
            NbtSummaryEntry {
                name: "click_event".into(),
                value: NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "action".into(),
                        value: NbtSummaryValue::string("change_page"),
                    },
                    NbtSummaryEntry {
                        name: "page".into(),
                        value: NbtSummaryValue::Int(page),
                    },
                ]),
            },
        ])
    }
}
