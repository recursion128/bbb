use super::TerrainSlot;
use std::ops::{Index, IndexMut};

const SLOTS_PER_PAGE: usize = 256;

// Slot IDs also index the GPU fade buffer. New pages extend the table without moving meshes.
#[derive(Default)]
pub(super) struct TerrainSlots {
    pages: Vec<Vec<Option<TerrainSlot>>>,
    free: Vec<u32>,
}

impl TerrainSlots {
    pub(super) fn allocate(&mut self) -> u32 {
        if let Some(slot) = self.free.pop() {
            return slot;
        }
        if self
            .pages
            .last()
            .is_none_or(|page| page.len() == SLOTS_PER_PAGE)
        {
            self.pages.push(Vec::with_capacity(SLOTS_PER_PAGE));
        }
        let page_index = self.pages.len() - 1;
        let page = &mut self.pages[page_index];
        let slot = page_index * SLOTS_PER_PAGE + page.len();
        page.push(None);
        slot as u32
    }

    pub(super) fn remove(&mut self, slot: u32) -> Option<TerrainSlot> {
        self.free.push(slot);
        self[slot as usize].take()
    }

    pub(super) fn occupied(&self) -> impl Iterator<Item = &TerrainSlot> {
        self.pages.iter().flatten().flatten()
    }

    pub(super) fn occupied_mut(&mut self) -> impl Iterator<Item = &mut TerrainSlot> {
        self.pages.iter_mut().flatten().flatten()
    }
}

impl Index<usize> for TerrainSlots {
    type Output = Option<TerrainSlot>;

    fn index(&self, slot: usize) -> &Self::Output {
        &self.pages[slot / SLOTS_PER_PAGE][slot % SLOTS_PER_PAGE]
    }
}

impl IndexMut<usize> for TerrainSlots {
    fn index_mut(&mut self, slot: usize) -> &mut Self::Output {
        &mut self.pages[slot / SLOTS_PER_PAGE][slot % SLOTS_PER_PAGE]
    }
}
