use super::*;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID as INSTRUMENT,
    VANILLA_DATA_COMPONENT_OMINOUS_BOTTLE_AMPLIFIER_TYPE_ID as OMINOUS_BOTTLE_AMPLIFIER,
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID as POTION_CONTENTS,
};
use bbb_protocol::static_registry_elements::vanilla_static_registry_elements;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::loot::HolderSet;

pub(super) const INSTRUMENT_REGISTRY: &str = "minecraft:instrument";
pub(super) const POTION_REGISTRY: &str = "minecraft:potion";

impl Eval<'_> {
    /// `set_instrument`, `set_random_potion` and `set_potion`: one member of
    /// `options` (the whole registry when absent) becomes the holder. The draw
    /// happens even for an empty stack; an empty set draws nothing.
    pub(super) fn apply_holder_component(
        &mut self,
        registry: &'static str,
        options: Option<&HolderSet>,
        draw: bool,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let environment = self.environment(path)?;
        let stack = RuntimeItemStack::from_summary(stack);
        let members = self.at(
            path,
            holder_members(registry, options, environment.item_resolver()),
        )?;
        if members.is_empty() {
            return Ok(stack.into_summary());
        }
        let index = if draw {
            self.random
                .next_int(i32::try_from(members.len()).unwrap_or(i32::MAX)) as usize
        } else {
            0
        };
        let holder = NbtSummaryValue::string(members[index].clone());
        let result = if registry == INSTRUMENT_REGISTRY {
            environment.with_component_runtime(stack, INSTRUMENT, Some(&holder))
        } else if stack.is_empty() {
            Ok(stack)
        } else {
            // `PotionContents.withPotion` keeps color, effects and name.
            let mut fields =
                match self.at(path, environment.component_runtime(&stack, POTION_CONTENTS))? {
                    Some(NbtSummaryValue::Compound(fields)) => fields,
                    None => Vec::new(),
                    Some(_) => unreachable!("decoded potion contents"),
                };
            fields.retain(|field| field.name != "potion");
            fields.push(NbtSummaryEntry {
                name: "potion".into(),
                value: holder,
            });
            let value = NbtSummaryValue::Compound(fields);
            environment.with_component_runtime(stack, POTION_CONTENTS, Some(&value))
        };
        self.at(path, result).map(RuntimeItemStack::into_summary)
    }

    /// No item or empty-stack guard.
    pub(super) fn apply_ominous_bottle_amplifier(
        &mut self,
        amplifier: &NumberProvider,
        stack: LootStack,
        path: &str,
    ) -> Result<LootStack> {
        let amplifier = self
            .number_int(amplifier, &format!("{path}.amplifier"))?
            .clamp(0, 4);
        self.write_component(
            stack,
            OMINOUS_BOTTLE_AMPLIFIER,
            Some(&NbtSummaryValue::Int(amplifier)),
            path,
        )
    }
}

/// The set's members in `HolderSet` order; no set is the registry in id order.
pub(super) fn holder_members(
    registry: &'static str,
    options: Option<&HolderSet>,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<Vec<String>, LootRuntimeReason> {
    match options {
        None => Ok(vanilla_static_registry_elements(registry)
            .ok_or_else(|| catalog_reason(format!("no builtin registry {registry}")))?
            .iter()
            .map(|id| (*id).to_string())
            .collect()),
        Some(HolderSet::Ids(ids)) => Ok(ids.clone()),
        Some(HolderSet::Tag(tag)) => match resolver.tag_members(registry, tag) {
            bbb_protocol::PersistentRegistryTagMembers::Present(members) => Ok(members),
            _ => Err(catalog_reason(format!("unbound tag #{tag}"))),
        },
    }
}
