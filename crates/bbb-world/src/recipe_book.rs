use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    RecipeBookAdd as ProtocolRecipeBookAdd, RecipeBookRemove as ProtocolRecipeBookRemove,
    RecipeBookSettings as ProtocolRecipeBookSettings, RecipeBookType,
    RecipeBookTypeSettings as ProtocolRecipeBookTypeSettings,
    RecipeDisplayEntry as ProtocolRecipeDisplayEntry,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

const RECIPE_BOOK_TAB_ANIMATION_TICKS: u32 = 15;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecipeBookState {
    pub known: BTreeMap<i32, ProtocolRecipeDisplayEntry>,
    pub highlights: BTreeSet<i32>,
    pub settings: ProtocolRecipeBookSettings,
    pub notification_ids: Vec<i32>,
    #[serde(default)]
    pub tab_animation_ticks_remaining: u32,
}

impl WorldStore {
    pub fn apply_recipe_book_add(&mut self, packet: ProtocolRecipeBookAdd) {
        self.counters.recipe_book_add_packets += 1;
        self.counters.recipe_book_entries_received += packet.entries.len();
        if packet.replace {
            self.counters.recipe_book_replace_packets += 1;
            self.recipe_book.known.clear();
            self.recipe_book.highlights.clear();
            self.recipe_book.tab_animation_ticks_remaining = 0;
        }

        self.recipe_book.notification_ids.clear();
        let mut highlighted = false;
        for entry in packet.entries {
            let id = entry.contents.id.index;
            if entry.highlight {
                self.recipe_book.highlights.insert(id);
                highlighted = true;
            }
            if entry.notification {
                self.counters.recipe_book_notifications_received += 1;
                self.recipe_book.notification_ids.push(id);
            }
            self.recipe_book.known.insert(id, entry.contents);
        }
        if highlighted {
            self.recipe_book.tab_animation_ticks_remaining = RECIPE_BOOK_TAB_ANIMATION_TICKS;
        }
        self.update_recipe_book_counts();
    }

    pub fn apply_recipe_book_remove(&mut self, packet: ProtocolRecipeBookRemove) {
        self.counters.recipe_book_remove_packets += 1;
        self.counters.recipe_book_removed_entries_received += packet.recipe_ids.len();
        for id in packet.recipe_ids {
            self.recipe_book.known.remove(&id.index);
            self.recipe_book.highlights.remove(&id.index);
        }
        if self.recipe_book.highlights.is_empty() {
            self.recipe_book.tab_animation_ticks_remaining = 0;
        }
        self.update_recipe_book_counts();
    }

    pub fn advance_recipe_book_tab_animation(&mut self, ticks: u32) {
        self.recipe_book.tab_animation_ticks_remaining = self
            .recipe_book
            .tab_animation_ticks_remaining
            .saturating_sub(ticks);
    }

    pub fn apply_recipe_book_settings(&mut self, settings: ProtocolRecipeBookSettings) {
        self.counters.recipe_book_settings_packets += 1;
        self.recipe_book.settings = settings;
    }

    pub fn set_local_recipe_book_type_settings(
        &mut self,
        book_type: RecipeBookType,
        settings: ProtocolRecipeBookTypeSettings,
    ) {
        match book_type {
            RecipeBookType::Crafting => self.recipe_book.settings.crafting = settings,
            RecipeBookType::Furnace => self.recipe_book.settings.furnace = settings,
            RecipeBookType::BlastFurnace => self.recipe_book.settings.blast_furnace = settings,
            RecipeBookType::Smoker => self.recipe_book.settings.smoker = settings,
        }
    }

    pub fn recipe_book(&self) -> &ClientRecipeBookState {
        &self.recipe_book
    }

    fn update_recipe_book_counts(&mut self) {
        self.counters.recipe_book_entries_tracked = self.recipe_book.known.len();
        self.counters.recipe_book_highlights_tracked = self.recipe_book.highlights.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        IngredientSummary, RecipeBookAddEntry, RecipeBookTypeSettings, RecipeDisplayId,
        RecipeDisplaySummary, RecipeDisplayType,
    };

    #[test]
    fn recipe_book_add_remove_and_settings_follow_client_semantics() {
        let mut store = WorldStore::new();

        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: true,
            entries: vec![recipe_entry(7, true, true), recipe_entry(8, false, false)],
        });
        assert_eq!(store.recipe_book().known.len(), 2);
        assert!(store.recipe_book().highlights.contains(&7));
        assert_eq!(store.recipe_book().notification_ids, vec![7]);
        assert_eq!(
            store.recipe_book().tab_animation_ticks_remaining,
            RECIPE_BOOK_TAB_ANIMATION_TICKS
        );
        store.advance_recipe_book_tab_animation(7);
        assert_eq!(store.recipe_book().tab_animation_ticks_remaining, 8);
        store.advance_recipe_book_tab_animation(8);
        assert_eq!(store.recipe_book().tab_animation_ticks_remaining, 0);

        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: false,
            entries: vec![recipe_entry(9, false, true)],
        });
        assert_eq!(store.recipe_book().known.len(), 3);
        assert!(store.recipe_book().highlights.contains(&9));
        assert_eq!(store.recipe_book().notification_ids, Vec::<i32>::new());
        assert_eq!(
            store.recipe_book().tab_animation_ticks_remaining,
            RECIPE_BOOK_TAB_ANIMATION_TICKS
        );

        store.apply_recipe_book_remove(ProtocolRecipeBookRemove {
            recipe_ids: vec![RecipeDisplayId { index: 7 }, RecipeDisplayId { index: 9 }],
        });
        assert!(!store.recipe_book().known.contains_key(&7));
        assert!(!store.recipe_book().known.contains_key(&9));
        assert!(!store.recipe_book().highlights.contains(&7));
        assert!(!store.recipe_book().highlights.contains(&9));
        assert_eq!(store.recipe_book().tab_animation_ticks_remaining, 0);

        store.apply_recipe_book_settings(ProtocolRecipeBookSettings {
            crafting: RecipeBookTypeSettings {
                open: true,
                filtering: false,
            },
            furnace: RecipeBookTypeSettings {
                open: false,
                filtering: true,
            },
            blast_furnace: RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
            smoker: RecipeBookTypeSettings {
                open: false,
                filtering: false,
            },
        });
        assert!(store.recipe_book().settings.crafting.open);
        assert!(store.recipe_book().settings.furnace.filtering);

        let counters = store.counters();
        assert_eq!(counters.recipe_book_add_packets, 2);
        assert_eq!(counters.recipe_book_replace_packets, 1);
        assert_eq!(counters.recipe_book_remove_packets, 1);
        assert_eq!(counters.recipe_book_settings_packets, 1);
        assert_eq!(counters.recipe_book_entries_received, 3);
        assert_eq!(counters.recipe_book_removed_entries_received, 2);
        assert_eq!(counters.recipe_book_entries_tracked, 1);
        assert_eq!(counters.recipe_book_highlights_tracked, 0);
        assert_eq!(counters.recipe_book_notifications_received, 1);
    }

    #[test]
    fn local_recipe_book_type_settings_update_without_packet_counter() {
        let mut store = WorldStore::new();

        store.set_local_recipe_book_type_settings(
            RecipeBookType::Crafting,
            RecipeBookTypeSettings {
                open: true,
                filtering: true,
            },
        );

        assert!(store.recipe_book().settings.crafting.open);
        assert!(store.recipe_book().settings.crafting.filtering);
        assert_eq!(store.counters().recipe_book_settings_packets, 0);
    }

    fn recipe_entry(id: i32, notification: bool, highlight: bool) -> RecipeBookAddEntry {
        RecipeBookAddEntry {
            contents: ProtocolRecipeDisplayEntry {
                id: RecipeDisplayId { index: id },
                display: RecipeDisplaySummary {
                    display_type: RecipeDisplayType::Stonecutter,
                    raw_body: vec![3, 0, 0, 0],
                    crafting: None,
                    furnace: None,
                },
                group: None,
                category_id: 10,
                crafting_requirements: Some(vec![IngredientSummary {
                    tag: None,
                    item_ids: vec![42],
                }]),
            },
            flags: (u8::from(notification)) | (u8::from(highlight) << 1),
            notification,
            highlight,
        }
    }
}
