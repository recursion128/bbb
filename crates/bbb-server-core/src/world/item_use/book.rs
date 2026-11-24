//! `WrittenBookItem.use`, `WritableBookItem.use` and `KnowledgeBookItem.use`.

use std::borrow::Cow;

use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::packets::{InteractionHand, OpenBook};
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::UnavailableHolderResolver;
use bbb_server_types::item_stack::RuntimeItemStack;
use bbb_server_types::stats::StatKey;

use super::{runtime_item_in_hand, ItemUseContext, ItemUseResult};
use crate::commands::{CommandOrigin, CommandSource};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::inventory::MenuBroadcastMode;
use crate::world::{send, ServerWorld, WorldEvent};

fn award_item_used(world: &mut ServerWorld, context: &ItemUseContext) {
    if let Some(item_id) = context.item.item_id {
        world.levels[context.connection_id].award_stat(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            1,
        );
    }
}

/// `WrittenBookItem.use` (`WrittenBookItem.java:15-20`).
pub(super) mod written_book {
    use super::*;

    pub(in crate::world::item_use) fn use_item(
        world: &mut ServerWorld,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> ItemUseResult {
        world.open_item_gui(context.connection_id, context.hand, events);
        award_item_used(world, context);
        ItemUseResult::Success
    }
}

/// `WritableBookItem.use` (`WritableBookItem.java:16-21`): the client opens
/// its own editor, since the stack has no `written_book_content`.
pub(super) mod writable_book {
    use super::*;

    pub(in crate::world::item_use) fn use_item(
        world: &mut ServerWorld,
        context: &ItemUseContext,
        _events: &mut Vec<WorldEvent>,
    ) -> ItemUseResult {
        award_item_used(world, context);
        ItemUseResult::Success
    }
}

/// `KnowledgeBookItem.use` (`KnowledgeBookItem.java:27-54`).
pub(super) mod knowledge_book {
    use super::*;

    pub(in crate::world::item_use) fn use_item(
        world: &mut ServerWorld,
        context: &ItemUseContext,
        events: &mut Vec<WorldEvent>,
    ) -> ItemUseResult {
        let connection_id = context.connection_id;
        let held =
            runtime_item_in_hand(world.levels.player_at(connection_id), context.hand).clone();
        let recipes = held
            .effective_component(
                VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
                &UnavailableHolderResolver,
            )
            .ok()
            .flatten()
            .and_then(|patch| match patch {
                Cow::Owned(patch) => patch.recipe_keys,
                Cow::Borrowed(patch) => patch.recipe_keys.clone(),
            })
            .unwrap_or_default();
        // `itemStack.consume(1, player)`.
        if !world.levels[connection_id].abilities.instabuild {
            world.update_hand_stack(
                connection_id,
                context.hand,
                held.with_count(held.count() - 1),
            );
        }
        if recipes.is_empty() {
            return ItemUseResult::Fail;
        }
        let Some(datapacks) = world.cold.config.datapacks.clone() else {
            return ItemUseResult::Fail;
        };
        if let Some(missing) = recipes
            .iter()
            .find(|id| datapacks.recipes.recipe(id).is_none())
        {
            tracing::error!("Invalid recipe: {missing}");
            return ItemUseResult::Fail;
        }
        world.award_recipes(connection_id, &recipes, events);
        award_item_used(world, context);
        ItemUseResult::Success
    }
}

impl ServerWorld {
    /// `ServerPlayer.openItemGui` (`ServerPlayer.java:1383-1391`).
    fn open_item_gui(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) {
        let held = runtime_item_in_hand(self.levels.player_at(connection_id), hand).clone();
        let resolver = UnavailableHolderResolver;
        let Ok(Some(content)) = held.component_nbt(
            VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
            &resolver,
        ) else {
            return;
        };
        if let Some(resolved) = self.resolve_written_book(connection_id, &content) {
            if let Ok(stack) = held.with_component_nbt(
                VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
                Some(&resolved),
                &resolver,
            ) {
                self.update_hand_stack(connection_id, hand, stack);
                self.broadcast_player_menu(connection_id, MenuBroadcastMode::Changes, events);
            }
        }
        events.push(send(
            connection_id,
            ClientboundPlayPacket::OpenBook(OpenBook { hand }),
        ));
    }

    /// `WrittenBookContent.resolveForItem` (`WrittenBookContent.java:92-105`)
    /// with the player's own command source: every page resolved against it,
    /// a page that fails kept as written, and the content marked resolved.
    /// `None` when it already was.
    fn resolve_written_book(
        &mut self,
        connection_id: ConnectionId,
        content: &NbtSummaryValue,
    ) -> Option<NbtSummaryValue> {
        let player = self.levels.player_at(connection_id);
        let source = CommandSource::player(
            &player.profile.name,
            player.profile.uuid,
            self.player_permission(connection_id),
        );
        let (position, rotation) = (player.position(), player.rotation());
        let origin = CommandOrigin {
            position: [position.x, position.y, position.z],
            rotation: [rotation.y_rot, rotation.x_rot],
            level: self.level_key(player.dimension).clone(),
            anchor: EntityAnchor::Feet,
            eye_height: None,
        };
        let recipient_uuid = player.profile.uuid;
        self.resolve_written_book_content(content, &source, &origin, Some(recipient_uuid))
    }

    /// `WrittenBookContent.resolveForItem` (`WrittenBookContent.java:92-105`)
    /// over one stack; `None` when it holds no unresolved content.
    pub(in crate::world) fn resolve_written_book_stack(
        &mut self,
        stack: &RuntimeItemStack,
        source: &CommandSource,
        origin: &CommandOrigin,
        recipient: Option<uuid::Uuid>,
    ) -> Option<RuntimeItemStack> {
        let resolver = UnavailableHolderResolver;
        let content = stack
            .component_nbt(
                VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
                &resolver,
            )
            .ok()??;
        let resolved = self.resolve_written_book_content(&content, source, origin, recipient)?;
        stack
            .clone()
            .with_component_nbt(
                VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
                Some(&resolved),
                &resolver,
            )
            .ok()
    }

    fn resolve_written_book_content(
        &mut self,
        content: &NbtSummaryValue,
        source: &CommandSource,
        origin: &CommandOrigin,
        recipient: Option<uuid::Uuid>,
    ) -> Option<NbtSummaryValue> {
        let NbtSummaryValue::Compound(fields) = content else {
            return None;
        };
        let resolved = |entry: &NbtSummaryEntry| entry.name == "resolved";
        if fields
            .iter()
            .any(|entry| resolved(entry) && entry.value == NbtSummaryValue::Byte(1))
        {
            return None;
        }
        self.with_entity_reads(|reads| {
            let recipient =
                recipient.and_then(|uuid| reads.players().find(|player| player.id == uuid));
            // `WrittenBookContent.resolvePage` (`WrittenBookContent.java:130-141`):
            // one page whose resolved JSON passes 32767 chars leaves every page
            // as written.
            let too_large = std::cell::Cell::new(false);
            let resolve = |component: &NbtSummaryValue| match crate::commands::resolve_text(
                component, source, origin, reads, recipient,
            ) {
                Ok(resolved) => {
                    if page_too_large(&resolved) {
                        too_large.set(true);
                    }
                    resolved
                }
                Err(_) => component.clone(),
            };
            // `Filterable`: `{raw, filtered}` or the bare component.
            let resolve_page = |page: &NbtSummaryValue| match page {
                NbtSummaryValue::Compound(entries)
                    if entries.iter().any(|entry| entry.name == "raw") =>
                {
                    NbtSummaryValue::Compound(
                        entries
                            .iter()
                            .map(|entry| NbtSummaryEntry {
                                name: entry.name.clone(),
                                value: if entry.name == "raw" || entry.name == "filtered" {
                                    resolve(&entry.value)
                                } else {
                                    entry.value.clone()
                                },
                            })
                            .collect(),
                    )
                }
                page => resolve(page),
            };
            let mut fields: Vec<NbtSummaryEntry> = fields
                .iter()
                .filter(|entry| !resolved(entry))
                .map(|entry| NbtSummaryEntry {
                    name: entry.name.clone(),
                    value: match &entry.value {
                        NbtSummaryValue::List(pages) if entry.name == "pages" => {
                            let resolved = pages.iter().map(&resolve_page).collect();
                            if too_large.get() {
                                entry.value.clone()
                            } else {
                                NbtSummaryValue::List(resolved)
                            }
                        }
                        value => value.clone(),
                    },
                })
                .collect();
            fields.push(NbtSummaryEntry {
                name: "resolved".into(),
                value: NbtSummaryValue::Byte(1),
            });
            Some(NbtSummaryValue::Compound(fields))
        })
    }
}

/// `WrittenBookContent.isPageTooLarge` (`WrittenBookContent.java:143-146`).
fn page_too_large(page: &NbtSummaryValue) -> bool {
    serde_json::to_string(&bbb_protocol::nbt_summary_to_json_value(page))
        .is_ok_and(|json| json.encode_utf16().count() > 32767)
}
