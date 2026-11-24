use super::*;
use bbb_protocol::item_types::{VANILLA_ITEM_BOOK_ID, VANILLA_ITEM_ENCHANTED_BOOK_ID};
use bbb_protocol::packets::ItemEnchantmentSummary;

impl Eval<'_> {
    pub(super) fn apply_native_enchantments(
        &mut self,
        enchantments: &[(String, NumberProvider)],
        add: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let mut stack = RuntimeItemStack::from_summary(stack);
        // A book becomes an enchanted book before its component is read.
        if !stack.is_empty() && stack.item_id() == Some(VANILLA_ITEM_BOOK_ID) {
            stack = self.at(
                path,
                environment.with_item_runtime(stack, VANILLA_ITEM_ENCHANTED_BOOK_ID),
            )?;
        }
        let Some(values) = stack
            .enchantments(environment.item_resolver())
            .map_err(|error| self.catalog(path, error))?
        else {
            // An absent component evaluates no provider.
            return Ok(stack.into_summary());
        };
        let mut fields = values.into_owned();
        for (id, provider) in enchantments {
            let level = self.number_int(provider, &format!("{path}.enchantments.{id}"))?;
            let holder_id = environment
                .item_resolver()
                .holder_id("minecraft:enchantment", id)
                .ok_or_else(|| self.catalog(path, format!("unknown enchantment {id}")))?;
            set_enchantment_level(&mut fields, holder_id, level, add);
        }
        stack
            .with_enchantments(fields, environment.item_resolver())
            .map(RuntimeItemStack::into_summary)
            .map_err(|error| self.catalog(path, error))
    }
}

pub(crate) fn set_enchantment_level(
    fields: &mut Vec<ItemEnchantmentSummary>,
    holder_id: i32,
    level: i32,
    add: bool,
) {
    let position = fields.iter().position(|field| field.holder_id == holder_id);
    let base = if add {
        position.map_or(0, |index| fields[index].level)
    } else {
        0
    };
    let level = base.wrapping_add(level).clamp(0, 255);
    match (position, level) {
        (Some(index), 0) => {
            fields.remove(index);
        }
        (Some(index), level) => fields[index].level = level,
        (None, 0) => {}
        (None, level) => fields.push(ItemEnchantmentSummary { holder_id, level }),
    }
}

pub(crate) fn upgrade_enchantment_level(
    fields: &mut Vec<ItemEnchantmentSummary>,
    holder_id: i32,
    level: i32,
) {
    let level = level.clamp(0, 255);
    if level == 0 {
        return;
    }
    if let Some(current) = fields.iter_mut().find(|entry| entry.holder_id == holder_id) {
        current.level = current.level.max(level);
    } else {
        fields.push(ItemEnchantmentSummary { holder_id, level });
    }
}
