use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::packets::{
    RecipeBookAdd as ProtocolRecipeBookAdd, RecipeBookRemove as ProtocolRecipeBookRemove,
    RecipeBookSettings as ProtocolRecipeBookSettings, RecipeBookType,
    RecipeBookTypeSettings as ProtocolRecipeBookTypeSettings,
    RecipeDisplayEntry as ProtocolRecipeDisplayEntry,
};
#[cfg(test)]
use bbb_protocol::recipe_wire_types::VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID;
use serde::{Deserialize, Serialize};

use crate::{recipes::ingredient_holder_set_is_resolved, WorldStore};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecipeBookState {
    pub known: BTreeMap<i32, ProtocolRecipeDisplayEntry>,
    pub highlights: BTreeSet<i32>,
    pub settings: ProtocolRecipeBookSettings,
    pub notification_ids: Vec<i32>,
    #[serde(default)]
    pub revision: u64,
}

impl WorldStore {
    pub fn apply_recipe_book_add(&mut self, packet: ProtocolRecipeBookAdd) {
        self.counters.recipe_book_add_packets += 1;
        self.counters.recipe_book_entries_received += packet.entries.len();
        if !packet.entries.iter().all(|entry| {
            entry
                .contents
                .crafting_requirements
                .as_deref()
                .unwrap_or_default()
                .iter()
                .all(|ingredient| {
                    ingredient_holder_set_is_resolved(self.frozen_registry_access(), ingredient)
                })
        }) {
            return;
        }
        if packet.replace {
            self.counters.recipe_book_replace_packets += 1;
            self.recipe_book.known.clear();
            self.recipe_book.highlights.clear();
        }

        self.recipe_book.notification_ids.clear();
        for entry in packet.entries {
            let id = entry.contents.id.index;
            if entry.highlight {
                self.recipe_book.highlights.insert(id);
            }
            if entry.notification {
                self.counters.recipe_book_notifications_received += 1;
                self.recipe_book.notification_ids.push(id);
                let category_item = entry
                    .contents
                    .display
                    .crafting_station()
                    .and_then(|display| {
                        self.resolve_slot_display_first_stack(display)
                            .ok()
                            .flatten()
                    })
                    .unwrap_or_else(bbb_protocol::packets::ItemStackSummary::empty);
                let unlocked_item = entry
                    .contents
                    .display
                    .result()
                    .and_then(|display| {
                        self.resolve_slot_display_first_stack(display)
                            .ok()
                            .flatten()
                    })
                    .unwrap_or_else(bbb_protocol::packets::ItemStackSummary::empty);
                self.push_recipe_toast_event(id, category_item, unlocked_item);
            }
            self.recipe_book.known.insert(id, entry.contents);
        }
        self.recipe_book.revision = self.recipe_book.revision.saturating_add(1);
        self.update_recipe_book_counts();
    }

    pub fn apply_recipe_book_remove(&mut self, packet: ProtocolRecipeBookRemove) {
        self.counters.recipe_book_remove_packets += 1;
        self.counters.recipe_book_removed_entries_received += packet.recipe_ids.len();
        for id in packet.recipe_ids {
            self.recipe_book.known.remove(&id.index);
            self.recipe_book.highlights.remove(&id.index);
        }
        self.recipe_book.revision = self.recipe_book.revision.saturating_add(1);
        self.update_recipe_book_counts();
    }

    pub fn remove_recipe_highlight(
        &mut self,
        recipe: bbb_protocol::packets::RecipeDisplayId,
    ) -> bool {
        if !self.recipe_book.highlights.remove(&recipe.index) {
            return false;
        }
        self.update_recipe_book_counts();
        true
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
    use bbb_protocol::codec::Encoder;
    use bbb_protocol::packets::{
        CraftingRecipeDisplaySummary, FurnaceRecipeDisplaySummary, IngredientSummary,
        RecipeBookAddEntry, RecipeBookTypeSettings, RecipeDisplayId, RecipeDisplaySummary,
        RecipeDisplayType, SlotDisplaySummary, SmithingRecipeDisplaySummary,
        StonecutterRecipeDisplaySummary,
    };
    use bbb_protocol::recipe_wire_types::VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID;

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
        assert_eq!(store.recipe_book().revision, 1);

        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: false,
            entries: vec![recipe_entry(9, false, true)],
        });
        assert_eq!(store.recipe_book().known.len(), 3);
        assert!(store.recipe_book().highlights.contains(&9));
        assert_eq!(store.recipe_book().notification_ids, Vec::<i32>::new());
        assert_eq!(store.recipe_book().revision, 2);

        store.apply_recipe_book_remove(ProtocolRecipeBookRemove {
            recipe_ids: vec![RecipeDisplayId { index: 7 }, RecipeDisplayId { index: 9 }],
        });
        assert!(!store.recipe_book().known.contains_key(&7));
        assert!(!store.recipe_book().known.contains_key(&9));
        assert!(!store.recipe_book().highlights.contains(&7));
        assert!(!store.recipe_book().highlights.contains(&9));
        assert_eq!(store.recipe_book().revision, 3);

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
    fn local_recipe_highlight_removal_is_guarded_and_updates_counts() {
        let mut store = WorldStore::new();
        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: true,
            entries: vec![recipe_entry(7, false, true)],
        });

        assert!(store.remove_recipe_highlight(RecipeDisplayId { index: 7 }));
        assert!(!store.remove_recipe_highlight(RecipeDisplayId { index: 7 }));
        assert!(store.recipe_book().highlights.is_empty());
        assert_eq!(store.counters().recipe_book_highlights_tracked, 0);
        assert_eq!(store.recipe_book().revision, 1);
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

    #[test]
    fn recipe_book_add_resolves_all_ingredient_holder_sets_before_replacing_state() {
        let mut store = WorldStore::new();
        store.apply_update_tags(bbb_protocol::packets::UpdateTags {
            registries: vec![bbb_protocol::packets::RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![bbb_protocol::packets::TagNetworkPayload {
                    tag: "example:crafting_inputs".to_string(),
                    entries: vec![42, 43],
                }],
            }],
        });

        let mut tagged = recipe_entry(7, false, false);
        tagged.contents.crafting_requirements = Some(vec![IngredientSummary {
            tag: Some("example:crafting_inputs".to_string()),
            item_ids: Vec::new(),
        }]);
        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: true,
            entries: vec![tagged],
        });
        let accepted = store.recipe_book().clone();

        let mut missing_tag = recipe_entry(8, true, true);
        missing_tag.contents.crafting_requirements = Some(vec![IngredientSummary {
            tag: Some("example:missing".to_string()),
            item_ids: Vec::new(),
        }]);
        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: true,
            entries: vec![missing_tag],
        });
        assert_eq!(store.recipe_book(), &accepted);

        let mut invalid_direct = recipe_entry(9, true, true);
        invalid_direct.contents.crafting_requirements = Some(vec![IngredientSummary {
            tag: None,
            item_ids: vec![i32::MAX],
        }]);
        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: false,
            entries: vec![invalid_direct],
        });
        assert_eq!(store.recipe_book(), &accepted);
        assert_eq!(store.counters().recipe_book_add_packets, 3);
        assert_eq!(store.counters().recipe_book_replace_packets, 1);
    }

    #[test]
    fn recipe_notifications_snapshot_first_station_and_result_for_every_display_family() {
        let displays = vec![
            RecipeDisplaySummary {
                display_type: RecipeDisplayType::CraftingShapeless,
                raw_body: Vec::new(),
                crafting: Some(CraftingRecipeDisplaySummary::Shapeless {
                    ingredients: Vec::new(),
                    result: item_display(2),
                    crafting_station: item_display(1),
                }),
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
            RecipeDisplaySummary {
                display_type: RecipeDisplayType::CraftingShaped,
                raw_body: Vec::new(),
                crafting: Some(CraftingRecipeDisplaySummary::Shaped {
                    width: 1,
                    height: 1,
                    ingredients: vec![item_display(42)],
                    result: item_display(4),
                    crafting_station: item_display(3),
                }),
                furnace: None,
                stonecutter: None,
                smithing: None,
            },
            RecipeDisplaySummary {
                display_type: RecipeDisplayType::Furnace,
                raw_body: Vec::new(),
                crafting: None,
                furnace: Some(FurnaceRecipeDisplaySummary {
                    ingredient: item_display(42),
                    fuel: item_display(1),
                    result: item_display(6),
                    crafting_station: item_display(5),
                    duration: 200,
                    experience_bits: 0,
                }),
                stonecutter: None,
                smithing: None,
            },
            RecipeDisplaySummary {
                display_type: RecipeDisplayType::Stonecutter,
                raw_body: Vec::new(),
                crafting: None,
                furnace: None,
                stonecutter: Some(StonecutterRecipeDisplaySummary {
                    input: item_display(42),
                    result: item_display(8),
                    crafting_station: item_display(7),
                }),
                smithing: None,
            },
            RecipeDisplaySummary {
                display_type: RecipeDisplayType::Smithing,
                raw_body: Vec::new(),
                crafting: None,
                furnace: None,
                stonecutter: None,
                smithing: Some(SmithingRecipeDisplaySummary {
                    template: item_display(42),
                    base: item_display(1),
                    addition: item_display(2),
                    result: item_display(10),
                    crafting_station: item_display(9),
                }),
            },
        ];
        let entries = displays
            .into_iter()
            .enumerate()
            .map(|(index, display)| RecipeBookAddEntry {
                contents: ProtocolRecipeDisplayEntry {
                    id: RecipeDisplayId {
                        index: i32::try_from(index).unwrap(),
                    },
                    display,
                    group: None,
                    category_id: VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
                    crafting_requirements: Some(vec![IngredientSummary {
                        tag: None,
                        item_ids: vec![42],
                    }]),
                },
                flags: 1,
                notification: true,
                highlight: false,
            })
            .collect();
        let mut store = WorldStore::new();

        store.apply_recipe_book_add(ProtocolRecipeBookAdd {
            replace: true,
            entries,
        });

        let snapshots = store
            .client_toast_events()
            .iter()
            .map(|event| {
                let (recipe_id, category, unlocked) = event.recipe().expect("recipe toast");
                (
                    recipe_id,
                    category.item_id.expect("category item"),
                    unlocked.item_id.expect("unlocked item"),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            snapshots,
            vec![(0, 1, 2), (1, 3, 4), (2, 5, 6), (3, 7, 8), (4, 9, 10)]
        );
        assert_eq!(store.counters().recipe_book_notifications_received, 5);
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
                    stonecutter: None,
                    smithing: None,
                },
                group: None,
                category_id: VANILLA_RECIPE_BOOK_CATEGORY_STONECUTTER_ID,
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

    fn item_display(item_id: i32) -> SlotDisplaySummary {
        let mut payload = Encoder::new();
        payload.write_var_i32(VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID);
        payload.write_var_i32(item_id);
        SlotDisplaySummary {
            display_type_id: VANILLA_SLOT_DISPLAY_TYPE_ITEM_ID,
            raw_payload: payload.into_inner(),
            item_stack: Some(bbb_protocol::packets::ItemStackSummary {
                item_id: Some(item_id),
                count: 1,
                component_patch: Default::default(),
            }),
            tag: None,
        }
    }
}
