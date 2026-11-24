use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID as WRITABLE,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID as WRITTEN,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_datapack::loot::ListOperation;

const WRITTEN_MAX_PAGES: usize = i32::MAX as usize;
const WRITABLE_MAX_PAGES: usize = 100;

impl Eval<'_> {
    pub(super) fn apply_book_cover(
        &mut self,
        value: &NbtSummaryValue,
        retain: &[&'static str],
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        if stack.is_empty() {
            return Ok(stack.into_summary());
        }
        let NbtSummaryValue::Compound(mut fields) = value.clone() else {
            unreachable!("decoded book cover")
        };
        if let Some(old) = self.at(path, environment.component_runtime(&stack, WRITTEN))? {
            let NbtSummaryValue::Compound(old) = old else {
                unreachable!("decoded book component")
            };
            fields.retain(|field| !retain.iter().any(|name| field.name == *name));
            fields.extend(
                old.into_iter()
                    .filter(|field| retain.iter().any(|name| field.name == *name)),
            );
        }
        let value = NbtSummaryValue::Compound(fields);
        self.at(
            path,
            environment.with_component_runtime(stack, WRITTEN, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }

    pub(super) fn apply_book_pages(
        &mut self,
        component: i32,
        value: &NbtSummaryValue,
        mode: ListOperation,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let mut old = match self.at(path, environment.component_runtime(&stack, component))? {
            Some(NbtSummaryValue::Compound(fields)) => fields,
            None => Vec::new(),
            Some(_) => return Err(self.err(path, malformed("book", "component"))),
        };
        let NbtSummaryValue::Compound(mut fields) = value.clone() else {
            unreachable!("decoded book component")
        };
        let held = take_pages(&mut old);
        let replacement = take_pages(&mut fields);
        let max_size = if component == WRITABLE {
            WRITABLE_MAX_PAGES
        } else {
            WRITTEN_MAX_PAGES
        };
        // A written book drops `resolved` even when the operation keeps the old pages.
        for entry in old {
            if component == WRITTEN && entry.name == "resolved" {
                continue;
            }
            fields.retain(|field| field.name != entry.name);
            fields.push(entry);
        }
        fields.push(NbtSummaryEntry {
            name: "pages".into(),
            value: NbtSummaryValue::List(mode.apply(held, replacement, max_size)),
        });
        let value = NbtSummaryValue::Compound(fields);
        self.at(
            path,
            environment.with_component_runtime(stack, component, Some(&value)),
        )
        .map(RuntimeItemStack::into_summary)
    }
}

fn take_pages(fields: &mut Vec<NbtSummaryEntry>) -> Vec<NbtSummaryValue> {
    let Some(index) = fields.iter().position(|field| field.name == "pages") else {
        return Vec::new();
    };
    let NbtSummaryValue::List(pages) = fields.remove(index).value else {
        unreachable!("decoded book pages")
    };
    pages
}
