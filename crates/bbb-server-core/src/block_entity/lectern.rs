//! `LecternBlockEntity` (`LecternBlockEntity.java:33-257`).
//!
//! One book, the current page, and the analog signal those two produce.
//! `resolveBook` (`:177-184`) needs a command source, so the level runs it.

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::value_input::int_or;
use super::{entry, unique_field, BlockEntityItemLoad, BlockEntityItemSave};

const TAG_BOOK: &str = "Book";
const TAG_PAGE: &str = "Page";

/// `LecternBlockEntity` (`LecternBlockEntity.java:33-132`).
#[derive(Debug, Clone, PartialEq)]
pub struct LecternBlockEntity {
    book: RuntimeItemStack,
    page: i32,
    page_count: i32,
}

impl Default for LecternBlockEntity {
    fn default() -> Self {
        Self {
            book: RuntimeItemStack::empty(),
            page: 0,
            page_count: 0,
        }
    }
}

impl LecternBlockEntity {
    pub fn original_book(&self) -> RuntimeItemStack {
        self.book.clone()
    }

    pub fn set_original_book(&mut self, book: RuntimeItemStack) -> bbb_server_types::Result<()> {
        if self.book == book {
            return Ok(());
        }
        let page_count = page_count(&book)?;
        self.book = book;
        self.page = 0;
        self.page_count = page_count;
        Ok(())
    }

    pub(super) fn original_items(&self) -> &[RuntimeItemStack] {
        std::slice::from_ref(&self.book)
    }

    pub(super) fn original_items_mut(&mut self) -> &mut [RuntimeItemStack] {
        std::slice::from_mut(&mut self.book)
    }

    pub fn remove_book(&mut self) -> RuntimeItemStack {
        let book = std::mem::replace(&mut self.book, RuntimeItemStack::empty());
        self.on_book_removed();
        book
    }

    #[must_use]
    pub fn book(&self) -> &RuntimeItemStack {
        &self.book
    }

    #[must_use]
    pub const fn page(&self) -> i32 {
        self.page
    }

    #[must_use]
    pub const fn page_count(&self) -> i32 {
        self.page_count
    }

    /// `LecternBlockEntity.hasBook` (`LecternBlockEntity.java:138-140`).
    pub fn has_book(&self) -> bbb_server_types::Result<bool> {
        let book = &self.book;
        Ok(
            book.has_component(VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID)?
                || book.has_component(VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID)?,
        )
    }

    /// The book `resolveBook` answered for the one already held, which keeps
    /// its pages and so the page.
    pub fn set_resolved_book(&mut self, book: RuntimeItemStack) {
        self.book = book;
    }

    /// `LecternBlockEntity.setBook` (`:152-157`) after `resolveBook`.
    pub fn set_book(&mut self, book: RuntimeItemStack) -> bbb_server_types::Result<()> {
        let page_count = page_count(&book)?;
        self.book = book;
        self.page = 0;
        self.page_count = page_count;
        Ok(())
    }

    /// `LecternBlockEntity.setPage` (`:159-166`). Answers whether the page
    /// actually moved, which is what `LecternBlock.signalPageChange` needs.
    pub fn set_page(&mut self, page: i32) -> bool {
        let new_page = clamp_page(page, self.page_count);
        if new_page == self.page {
            return false;
        }
        self.page = new_page;
        true
    }

    /// `LecternBlockEntity.onBookItemRemove` (`:146-150`) as the record half.
    pub fn on_book_removed(&mut self) {
        self.page = 0;
        self.page_count = 0;
    }

    /// `LecternBlockEntity.getRedstoneSignal` (`:172-175`).
    pub fn redstone_signal(&self) -> bbb_server_types::Result<i32> {
        let page_progress = if self.page_count > 1 {
            self.page as f32 / (self.page_count as f32 - 1.0)
        } else {
            1.0
        };
        Ok((page_progress * 14.0).floor() as i32 + i32::from(self.has_book()?))
    }

    pub(super) fn save_additional(
        &self,
        save: &mut BlockEntityItemSave<'_>,
    ) -> bbb_server_types::Result<Vec<NbtSummaryEntry>> {
        if self.book.is_empty() {
            return Ok(Vec::new());
        }
        // `output.store("Book", ItemStack.CODEC, ...)` (`:215`) leaves the
        // field out when the encode fails -- `ItemStack.CODEC` never answers a
        // partial value for `TagValueOutput.store`'s partial arm
        // (`TagValueOutput.java:43`) -- while `putInt("Page", ...)` (`:216`)
        // runs either way.
        let mut entries = Vec::new();
        if let Some(book) = save.write(&self.book)? {
            entries.push(entry(TAG_BOOK, book));
        }
        entries.push(entry(TAG_PAGE, NbtSummaryValue::Int(self.page)));
        Ok(entries)
    }

    pub(super) fn load_additional(
        &mut self,
        record: &[NbtSummaryEntry],
        load: &mut BlockEntityItemLoad<'_>,
    ) -> bbb_server_types::Result<()> {
        let book = match unique_field(record, TAG_BOOK) {
            Some(value) => load_book(value, load)?.unwrap_or_else(RuntimeItemStack::empty),
            None => RuntimeItemStack::empty(),
        };
        let page_count = page_count(&book)?;
        self.book = book;
        self.page_count = page_count;
        let page = int_or(record, TAG_PAGE, 0);
        self.page = clamp_page(page, self.page_count);
        Ok(())
    }
}

/// `is(ItemTags.LECTERN_BOOKS)` (`LecternBlock.java:227`) against the level's tags.
#[must_use]
pub fn item_is_lectern_book(
    item: &RuntimeItemStack,
    tags: &crate::registry_tags::RegistryTagSnapshot,
) -> bool {
    item.count() > 0
        && item
            .item_id()
            .and_then(bbb_protocol::item_types::vanilla_item_type)
            .is_some_and(|entry| {
                tags.contains(
                    "minecraft:item",
                    "minecraft:lectern_books",
                    entry.resource_id,
                )
            })
}

fn page_count(book: &RuntimeItemStack) -> bbb_server_types::Result<i32> {
    let book = book.summary();
    if let Some(written) = &book.component_patch.written_book {
        return Ok(i32::try_from(written.pages.len()).unwrap_or(0));
    }
    Ok(i32::try_from(book.component_patch.writable_book_pages.len()).unwrap_or(0))
}

/// `Mth.clamp(page, 0, pageCount - 1)` (`Mth.java:93-95`).
fn clamp_page(page: i32, page_count: i32) -> i32 {
    page.max(0).min(page_count - 1)
}

fn load_book(
    value: &NbtSummaryValue,
    load: &mut BlockEntityItemLoad<'_>,
) -> bbb_server_types::Result<Option<RuntimeItemStack>> {
    load.read(value)
}

/// `LecternBlockEntity.loadAdditional`'s `Book` read (`:204-208`) over the
/// compound `BlockItem.updateCustomBlockEntityTag` merges into the lectern's
/// own record (`BlockItem.java:148-171`): the stack the field holds.
///
/// `None` where the compound is not one `ItemStack.CODEC` could be read from at
/// all, which leaves the caller's own default in place. The lectern-books tag
/// is not checked here: vanilla's read stores whatever the codec answered, and
/// `LecternBlockEntity.preRemoveSideEffects` (`LecternBlockEntity.java:226-236`):
/// the book leaves the object and lands one block above the lectern, a quarter
/// block toward its facing.
#[must_use]
pub fn removal_drops(
    lectern: &LecternBlockEntity,
    old_state: i32,
) -> super::BlockEntityRemovalDrops {
    if !crate::block_behavior::lectern::has_book(old_state) {
        return super::BlockEntityRemovalDrops::None;
    }
    let (step_x, step_z) = match crate::block_behavior::lectern::facing(old_state) {
        bbb_protocol::packets::Direction::East => (1.0, 0.0),
        bbb_protocol::packets::Direction::West => (-1.0, 0.0),
        bbb_protocol::packets::Direction::South => (0.0, 1.0),
        _ => (0.0, -1.0),
    };
    super::BlockEntityRemovalDrops::LecternBook {
        stack: lectern.book.clone(),
        offset: [0.5 + 0.25 * step_x, 1.0, 0.5 + 0.25 * step_z],
    }
}
