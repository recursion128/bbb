use super::*;
use crate::random::GameRandom;
use crate::world::ValueEffectLevel;
use bbb_registry_codec::PersistentItemStackHolderResolver;

/// Frozen item reads, with the live level data needed by enchantment predicates.
pub struct RegistryItemEnvironment<'a> {
    resolver: &'a dyn PersistentItemStackHolderResolver,
    value_level: &'a ValueEffectLevel<'a>,
}
impl<'a> RegistryItemEnvironment<'a> {
    #[cfg(test)]
    pub fn new(resolver: &'a dyn PersistentItemStackHolderResolver) -> Self {
        const CLEAR_LEVEL: ValueEffectLevel<'static> =
            ValueEffectLevel::weather_only(LootWeather {
                raining: false,
                thundering: false,
            });
        Self::for_enchantment(resolver, &CLEAR_LEVEL)
    }

    pub(crate) fn for_enchantment(
        resolver: &'a dyn PersistentItemStackHolderResolver,
        level: &'a ValueEffectLevel<'a>,
    ) -> Self {
        Self {
            resolver,
            value_level: level,
        }
    }
}
impl LootItemEnvironment for RegistryItemEnvironment<'_> {
    fn clock_total_ticks(&self, clock: &str) -> std::result::Result<i64, LootRuntimeReason> {
        self.value_level.clock_total_ticks(clock)
    }

    fn environment_attribute(
        &self,
        attribute: &'static VanillaEnvironmentAttribute,
        origin: Option<LootOrigin>,
    ) -> std::result::Result<LootEnvironmentAttributeValue<'_>, LootRuntimeReason> {
        self.value_level.environment_attribute(attribute, origin)
    }

    fn entity_for_relations<'s>(
        &'s self,
        entity: LootEntity<'_>,
        bodies: Option<crate::entity::store::EntityBodyReadView<'s>>,
    ) -> Option<StoredLootEntity<'s>> {
        self.value_level.entity_for_relations(entity, bodies)
    }

    fn nbt_entity(
        &self,
        entity: LootEntity<'_>,
    ) -> std::result::Result<NbtSummaryValue, LootRuntimeReason> {
        if entity.player_uuid().is_some() {
            return Err(LootRuntimeReason::Catalog(
                "player NBT requires the actual save owner".into(),
            ));
        }
        entity.nbt_data()
    }

    fn item_resolver(&self) -> &dyn PersistentItemStackHolderResolver {
        self.resolver
    }
    fn enchant_with_levels(
        &self,
        _: ItemStackSummary,
        _: i32,
        _: Option<&bbb_server_datapack::loot::HolderSet>,
        _: &GameRandom,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
        Err(unsupported_field(
            "enchant_with_levels",
            "snapshot has no level enchantment environment",
        ))
    }
    fn matches_item(
        &self,
        _: &bbb_server_datapack::loot::ItemPredicate,
        _: &ItemStackSummary,
    ) -> std::result::Result<bool, LootRuntimeReason> {
        Err(unsupported_field(
            "filtered",
            "snapshot has no level item tags",
        ))
    }
    fn smelt_result(
        &self,
        _: &ItemStackSummary,
    ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
        Err(unsupported_field(
            "furnace_smelt",
            "snapshot has no recipes",
        ))
    }
}
