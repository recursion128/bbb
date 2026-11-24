use super::*;
use bbb_server_datapack::loot::slot_source::LootSlotSource;

impl Checker<'_> {
    pub(super) fn slot_source(&mut self, source: &LootSlotSource, path: &str) -> Result<()> {
        match source {
            LootSlotSource::Range { source, .. } => self.referenced(path, *source),
            LootSlotSource::Group(terms) => {
                for (index, term) in terms.iter().enumerate() {
                    self.slot_source(term, &format!("{path}.terms[{index}]"))?;
                }
                Ok(())
            }
            LootSlotSource::Filtered {
                slot_source,
                item_filter,
            } => {
                self.at(path, check_item_predicate(item_filter))?;
                self.slot_source(slot_source, path)
            }
            LootSlotSource::Limit { slot_source, .. }
            | LootSlotSource::Contents { slot_source, .. } => self.slot_source(slot_source, path),
            LootSlotSource::Empty => Ok(()),
        }
    }
}

impl Eval<'_> {
    pub(super) fn slot_source_stacks(
        &self,
        source: &LootSlotSource,
        path: &str,
    ) -> Result<Vec<RuntimeItemStack>> {
        match source {
            LootSlotSource::Range { source, slots } => {
                if *source == LootContextParam::BlockEntity {
                    let Some(block) = self.context.block_entity() else {
                        return Ok(Vec::new());
                    };
                    let block = block.block_entity;
                    let Some(items) = block.loot_slots() else {
                        return Ok(Vec::new());
                    };
                    if block.crafter().is_none() && block.container_loot_table_id().is_some() {
                        if !slots
                            .iter()
                            .any(|slot| usize::try_from(*slot).is_ok_and(|slot| slot < items.len()))
                        {
                            return Ok(Vec::new());
                        }
                        return self.at(
                            path,
                            self.environment(path)?.block_entity_slots(block, slots),
                        );
                    }
                    return Ok(slots
                        .iter()
                        .filter_map(|slot| usize::try_from(*slot).ok().and_then(|i| items.get(i)))
                        .cloned()
                        .collect());
                }
                let entity = self.context.entity(*source);
                if !entity.present() {
                    return Ok(Vec::new());
                }
                let entity = match entity {
                    LootEntity::Facts(facts)
                        if facts.entity_uuid.is_some() && facts.player_uuid.is_none() =>
                    {
                        self.context
                            .item_environment()
                            .and_then(|environment| environment.entity_for_relations(entity, None))
                            .map_or(entity, LootEntity::Stored)
                    }
                    _ => entity,
                };
                if let LootEntity::Stored(stored) = entity {
                    if crate::entity::item_slots::container_has_loot(stored.entity) {
                        let size = crate::entity::item_slots::container_size(stored.entity)
                            .expect("a pending vehicle has cargo slots");
                        if !slots
                            .iter()
                            .any(|slot| usize::try_from(*slot).is_ok_and(|slot| slot < size))
                        {
                            return Ok(Vec::new());
                        }
                        let read = self.context.vehicle_slots().ok_or_else(|| {
                            self.err(
                                path,
                                unsupported_field("slots", "unpack vehicle loot at its level"),
                            )
                        })?;
                        return self.at(path, read(stored.entity, slots));
                    }
                }
                let mut stacks = Vec::new();
                for slot in *slots {
                    if let Some((tool, _)) = self.at(
                        path,
                        entity.slot_item(*slot, self.context.item_environment()),
                    )? {
                        stacks.push(tool.stack);
                    }
                }
                Ok(stacks)
            }
            LootSlotSource::Group(terms) => {
                let mut stacks = Vec::new();
                for term in terms {
                    stacks.extend(self.slot_source_stacks(term, path)?);
                }
                Ok(stacks)
            }
            LootSlotSource::Filtered {
                slot_source,
                item_filter,
            } => {
                let stacks = self.slot_source_stacks(slot_source, path)?;
                if stacks.is_empty() {
                    return Ok(stacks);
                }
                let resolver = self.environment(path)?.item_resolver();
                let mut filtered = Vec::with_capacity(stacks.len());
                for stack in stacks {
                    let tool = LootTool {
                        item_id: stack.item_id(),
                        count: stack.count(),
                        stack,
                        ..Default::default()
                    };
                    if self.at(path, item_predicate_matches(item_filter, &tool, resolver))? {
                        filtered.push(tool.stack);
                    }
                }
                Ok(filtered)
            }
            LootSlotSource::Limit { slot_source, limit } => {
                // Valid empty slots count toward the limit before SlotLoot removes empty items.
                let mut stacks = self.slot_source_stacks(slot_source, path)?;
                stacks.truncate(*limit);
                Ok(stacks)
            }
            LootSlotSource::Contents {
                slot_source,
                component,
            } => {
                let stacks = self.slot_source_stacks(slot_source, path)?;
                if stacks.is_empty() {
                    return Ok(stacks);
                }
                let environment = self.environment(path)?;
                let mut contents = Vec::new();
                for stack in stacks {
                    if let Some(items) =
                        self.at(path, environment.contents_runtime(&stack, *component))?
                    {
                        // ContainerComponentManipulator.getSlots omits empty children.
                        contents.extend(items.into_iter().filter(|item| !item.is_empty()));
                    }
                }
                Ok(contents)
            }
            LootSlotSource::Empty => Ok(Vec::new()),
        }
    }
}
