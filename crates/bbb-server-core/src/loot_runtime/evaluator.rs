use super::*;
use crate::random::GameRandom;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_BUNDLE_CONTENTS_TYPE_ID as BUNDLE_CONTENTS,
    VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID as CONTAINER,
};
/// The last `name` field of a compound.
pub(crate) use bbb_protocol::nbt_summary::value_last_field as nbt_field;
use bbb_protocol::packets::DataComponentPatchSummary;
use bbb_protocol::NbtSummaryValue;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::loot::{
    ComponentPatchParameter, EnvironmentAttributeExpectedValue, HolderSet, ItemPredicate,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootRandomEnchantment {
    pub id: String,
    pub min_level: i32,
    pub max_level: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LootEnvironmentAttributeValue<'a> {
    Scalar(VanillaEnvironmentScalarValue),
    Integer(i32),
    Color(u32),
    Activity(&'a str),
    MoonPhase(bbb_server_types::environment::MoonPhase),
    TriState(bbb_server_types::environment::TriState),
    BedRule(&'a bbb_server_types::environment::BedRule),
    BackgroundMusic(&'a bbb_protocol::environment_audio::BackgroundMusic),
    AmbientSounds(&'a bbb_protocol::environment_audio::AmbientSounds),
    Particle(&'a bbb_protocol::environment_particles::ParticleOptions),
    AmbientParticles(&'a [bbb_protocol::environment_particles::AmbientParticle]),
}

pub(super) fn environment_attribute_expected<'a>(
    _attribute: &'static VanillaEnvironmentAttribute,
    value: &'a EnvironmentAttributeExpectedValue,
) -> std::result::Result<LootEnvironmentAttributeValue<'a>, LootRuntimeReason> {
    match value {
        EnvironmentAttributeExpectedValue::Scalar(value) => {
            Ok(LootEnvironmentAttributeValue::Scalar(*value))
        }
        EnvironmentAttributeExpectedValue::Integer(value) => {
            Ok(LootEnvironmentAttributeValue::Integer(*value))
        }
        EnvironmentAttributeExpectedValue::Color(value) => {
            Ok(LootEnvironmentAttributeValue::Color(*value))
        }
        EnvironmentAttributeExpectedValue::Activity(value) => {
            Ok(LootEnvironmentAttributeValue::Activity(value.resource_id))
        }
        EnvironmentAttributeExpectedValue::MoonPhase(value) => {
            Ok(LootEnvironmentAttributeValue::MoonPhase(*value))
        }
        EnvironmentAttributeExpectedValue::TriState(value) => {
            Ok(LootEnvironmentAttributeValue::TriState(*value))
        }
        EnvironmentAttributeExpectedValue::BedRule(value) => {
            Ok(LootEnvironmentAttributeValue::BedRule(value))
        }
        EnvironmentAttributeExpectedValue::BackgroundMusic(value) => {
            Ok(LootEnvironmentAttributeValue::BackgroundMusic(value))
        }
        EnvironmentAttributeExpectedValue::AmbientSounds(value) => {
            Ok(LootEnvironmentAttributeValue::AmbientSounds(value))
        }
        EnvironmentAttributeExpectedValue::Particle(value) => {
            Ok(LootEnvironmentAttributeValue::Particle(value))
        }
        EnvironmentAttributeExpectedValue::AmbientParticles(value) => {
            Ok(LootEnvironmentAttributeValue::AmbientParticles(value))
        }
    }
}

pub(super) fn environment_attribute_equal(
    expected: LootEnvironmentAttributeValue<'_>,
    actual: LootEnvironmentAttributeValue<'_>,
) -> std::result::Result<bool, LootRuntimeReason> {
    use LootEnvironmentAttributeValue::{
        Activity, AmbientParticles, AmbientSounds, BackgroundMusic, BedRule, Color, Integer,
        MoonPhase, Particle, Scalar, TriState,
    };
    use VanillaEnvironmentScalarValue as V;
    Ok(match (expected, actual) {
        (BedRule(expected), BedRule(actual)) => {
            if expected.can_sleep != actual.can_sleep
                || expected.can_set_spawn != actual.can_set_spawn
                || expected.explodes != actual.explodes
            {
                return Ok(false);
            }
            match (&expected.error_message, &actual.error_message) {
                (None, None) => true,
                (Some(expected), Some(actual)) => {
                    return bbb_protocol::component_codec_values_equal(expected, actual)
                        .map_err(catalog_reason);
                }
                _ => false,
            }
        }
        (Integer(expected), Integer(actual)) => expected == actual,
        (Color(expected), Color(actual)) => expected == actual,
        (Activity(expected), Activity(actual)) => expected == actual,
        (BackgroundMusic(expected), BackgroundMusic(actual)) => expected == actual,
        (AmbientSounds(expected), AmbientSounds(actual)) => expected == actual,
        (Particle(expected), Particle(actual)) => particle_equal(expected, actual),
        (AmbientParticles(expected), AmbientParticles(actual)) => {
            expected.len() == actual.len()
                && expected.iter().zip(actual).all(|(expected, actual)| {
                    expected.probability_bits == actual.probability_bits
                        && particle_equal(&expected.particle, &actual.particle)
                })
        }
        (MoonPhase(expected), MoonPhase(actual)) => expected == actual,
        (TriState(expected), TriState(actual)) => expected == actual,
        (Scalar(V::Boolean(expected)), Scalar(V::Boolean(actual))) => expected == actual,
        (Scalar(V::FloatBits(expected)), Scalar(V::FloatBits(actual))) => {
            expected == actual
                || (f32::from_bits(expected).is_nan() && f32::from_bits(actual).is_nan())
        }
        _ => false,
    })
}

fn particle_equal(
    expected: &bbb_protocol::environment_particles::ParticleOptions,
    actual: &bbb_protocol::environment_particles::ParticleOptions,
) -> bool {
    expected.resource_id == actual.resource_id && expected.option == actual.option
}

pub trait LootItemEnvironment {
    fn biome_at(
        &self,
        _pos: bbb_protocol::packets::BlockPos,
    ) -> std::result::Result<Option<&str>, LootRuntimeReason> {
        Err(LootRuntimeReason::NoLevelView)
    }

    fn light_at(
        &self,
        _pos: bbb_protocol::packets::BlockPos,
    ) -> std::result::Result<Option<i32>, LootRuntimeReason> {
        Err(LootRuntimeReason::NoLevelView)
    }

    fn structure_matches(
        &self,
        _pos: bbb_protocol::packets::BlockPos,
        _expected: &bbb_server_datapack::loot::HolderSet,
    ) -> std::result::Result<Option<bool>, LootRuntimeReason> {
        Err(LootRuntimeReason::NoLevelView)
    }

    fn sky_at(
        &self,
        _pos: bbb_protocol::packets::BlockPos,
    ) -> std::result::Result<Option<bool>, LootRuntimeReason> {
        Err(LootRuntimeReason::NoLevelView)
    }

    fn environment_attribute(
        &self,
        _attribute: &'static VanillaEnvironmentAttribute,
        _origin: Option<LootOrigin>,
    ) -> std::result::Result<LootEnvironmentAttributeValue<'_>, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "environment attribute requires the actual level environment".into(),
        ))
    }

    fn score(
        &self,
        _name: &bbb_protocol::JavaString,
        _objective: &bbb_protocol::JavaString,
    ) -> std::result::Result<Option<i32>, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "score requires the actual scoreboard".into(),
        ))
    }

    fn entity_score(
        &self,
        _entity: LootEntity<'_>,
        _objective: &bbb_protocol::JavaString,
        _bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    ) -> std::result::Result<Option<i32>, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "entity score requires the actual scoreboard".into(),
        ))
    }

    fn clock_total_ticks(&self, _clock: &str) -> std::result::Result<i64, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "time condition requires actual world clocks".into(),
        ))
    }

    fn entity_team_matches(
        &self,
        _entity: LootEntity<'_>,
        _expected: &bbb_protocol::JavaString,
        _bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
    ) -> std::result::Result<bool, LootRuntimeReason> {
        Err(unsupported_field(
            "entity_properties",
            "predicate.team needs the actual scoreboard",
        ))
    }

    fn entity_for_relations<'s>(
        &'s self,
        _entity: LootEntity<'_>,
        _bodies: Option<crate::entity::store::EntityBodyReadView<'s>>,
    ) -> Option<StoredLootEntity<'s>> {
        None
    }

    /// `PlayerPredicate`'s stats, recipes, advancements and input terms
    /// (`PlayerPredicate.java:90-135`) for the online player `uuid`.
    fn player_terms_match(
        &self,
        _uuid: uuid::Uuid,
        _terms: &bbb_server_datapack::loot::PlayerTerms,
    ) -> std::result::Result<bool, LootRuntimeReason> {
        Err(unsupported_field(
            "entity_properties",
            "predicate.type_specific minecraft:player needs the online player's records",
        ))
    }

    /// The entity `PlayerPredicate.looking_at` tests (`PlayerPredicate.java:118-133`):
    /// the nearest entity on the player's 100-block view ray that it has line of
    /// sight to.
    fn player_looking_at<'s>(
        &'s self,
        _uuid: uuid::Uuid,
    ) -> std::result::Result<Option<StoredLootEntity<'s>>, LootRuntimeReason> {
        Err(unsupported_field(
            "entity_properties",
            "predicate.type_specific minecraft:player looking_at needs the player's level",
        ))
    }

    /// The publication holder ids in item components resolve against.
    fn item_resolver(&self) -> &dyn PersistentItemStackHolderResolver {
        &bbb_registry_codec::UnavailableHolderResolver
    }

    fn player_item_slot(
        &self,
        _uuid: uuid::Uuid,
        _slot: i32,
    ) -> std::result::Result<Option<LootTool>, LootRuntimeReason> {
        Err(unsupported_field(
            "entity_properties",
            "predicate.slots needs an online player inventory",
        ))
    }

    fn block_entity_slots(
        &self,
        _original: &crate::block_entity::BlockEntity,
        _slots: &[i32],
    ) -> std::result::Result<Vec<RuntimeItemStack>, LootRuntimeReason> {
        Err(unsupported_field(
            "slots",
            "unpack container loot at its level",
        ))
    }

    fn exploration_map(
        &self,
        _origin: LootOrigin,
        _options: &LootExplorationMap,
    ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason> {
        Err(unsupported_field(
            "exploration_map",
            "actual structure location and map saved-data environment unavailable",
        ))
    }

    fn nbt_storage(&self, _id: &str) -> std::result::Result<NbtSummaryValue, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "NBT storage requires actual command storage".into(),
        ))
    }

    fn nbt_entity(
        &self,
        _entity: LootEntity<'_>,
    ) -> std::result::Result<NbtSummaryValue, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "NBT entity requires actual saveWithoutId output".into(),
        ))
    }

    fn nbt_block_entity(
        &self,
        _entity: &crate::block_entity::BlockEntity,
    ) -> std::result::Result<NbtSummaryValue, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "NBT block entity requires actual full metadata output".into(),
        ))
    }

    fn mob_effect_instantaneous(&self, _id: &str) -> std::result::Result<bool, LootRuntimeReason> {
        Err(LootRuntimeReason::Catalog(
            "mob effect read requires actual effect environment".into(),
        ))
    }

    /// `ItemStack.transmuteCopy` applies the patch against the target prototype.
    fn with_item_runtime(
        &self,
        stack: RuntimeItemStack,
        item_id: i32,
    ) -> std::result::Result<RuntimeItemStack, LootRuntimeReason> {
        transmute_runtime_item(&stack, item_id, self.item_resolver())
    }

    fn component_runtime(
        &self,
        stack: &RuntimeItemStack,
        type_id: i32,
    ) -> std::result::Result<Option<NbtSummaryValue>, LootRuntimeReason> {
        stack
            .component_nbt(type_id, self.item_resolver())
            .map_err(catalog_reason)
    }

    fn with_component_runtime(
        &self,
        stack: RuntimeItemStack,
        type_id: i32,
        value: Option<&NbtSummaryValue>,
    ) -> std::result::Result<RuntimeItemStack, LootRuntimeReason> {
        stack
            .with_component_nbt(type_id, value, self.item_resolver())
            .map_err(catalog_reason)
    }

    fn contents_runtime(
        &self,
        stack: &RuntimeItemStack,
        type_id: i32,
    ) -> std::result::Result<Option<Vec<RuntimeItemStack>>, LootRuntimeReason> {
        if type_id == CONTAINER {
            return Ok(stack.container_contents().map(|mut items| {
                items.truncate(
                    items
                        .iter()
                        .rposition(|item| !item.is_empty())
                        .map_or(0, |slot| slot + 1),
                );
                items
            }));
        }
        let Some(patch) = stack
            .effective_component(type_id, self.item_resolver())
            .map_err(catalog_reason)?
        else {
            return Ok(None);
        };
        let items = if type_id == BUNDLE_CONTENTS {
            &patch.bundle_contents_items
        } else {
            &patch.charged_projectiles_items
        };
        Ok(Some(
            items.iter().map(RuntimeItemStack::from_template).collect(),
        ))
    }

    fn with_contents_runtime(
        &self,
        stack: RuntimeItemStack,
        type_id: i32,
        contents: Vec<RuntimeItemStack>,
    ) -> std::result::Result<RuntimeItemStack, LootRuntimeReason> {
        let patch = if type_id == CONTAINER {
            bbb_server_types::item_stack::container_patch(&contents)
        } else {
            let items: Vec<_> = contents
                .iter()
                .filter_map(RuntimeItemStack::to_template)
                .collect();
            let mut patch = DataComponentPatchSummary {
                added_type_ids: vec![type_id],
                ..Default::default()
            };
            if type_id == BUNDLE_CONTENTS {
                patch.bundle_contents_items = items;
            } else {
                patch.charged_projectiles_items = items;
            }
            patch
        };
        bbb_protocol::packets::data_components::validate_persistent_data_component_patch_summary(
            &patch,
        )
        .map_err(catalog_reason)?;
        stack
            .with_component_patch(&patch, self.item_resolver())
            .map_err(catalog_reason)
    }

    /// `ItemStack.applyComponents` with an already decoded patch.
    fn with_component_patch(
        &self,
        stack: LootStack,
        patch: &DataComponentPatchSummary,
    ) -> std::result::Result<LootStack, LootRuntimeReason> {
        let stack = RuntimeItemStack::from_summary(stack);
        stack
            .with_component_patch(patch, self.item_resolver())
            .map(RuntimeItemStack::into_summary)
            .map_err(catalog_reason)
    }

    /// `EnchantRandomlyFunction.run`: options keep holder order; absent
    /// options use registry order. Books bypass compatibility, as does
    /// `only_compatible == false`.
    fn random_enchantment_candidates(
        &self,
        _stack: &ItemStackSummary,
        _options: Option<&HolderSet>,
        _only_compatible: bool,
    ) -> std::result::Result<Vec<LootRandomEnchantment>, LootRuntimeReason> {
        Err(unsupported_field(
            "enchant_randomly",
            "enchantment registry environment",
        ))
    }

    /// `EnchantRandomlyFunction.enchantItem`: a book becomes a fresh enchanted
    /// book before the selected enchantment.
    fn with_random_enchantment(
        &self,
        _stack: ItemStackSummary,
        _enchantment: &LootRandomEnchantment,
        _level: i32,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason> {
        Err(unsupported_field(
            "enchant_randomly",
            "enchantment item write environment",
        ))
    }

    fn enchant_with_levels(
        &self,
        stack: ItemStackSummary,
        levels: i32,
        options: Option<&HolderSet>,
        random: &GameRandom,
    ) -> std::result::Result<ItemStackSummary, LootRuntimeReason>;

    fn matches_item(
        &self,
        predicate: &ItemPredicate,
        stack: &ItemStackSummary,
    ) -> std::result::Result<bool, LootRuntimeReason>;

    /// The smelting result for a one-item input. A miss is `Ok(None)`, which
    /// leaves the stack; an environment with no recipe manager refuses.
    fn smelt_result(
        &self,
        stack: &ItemStackSummary,
    ) -> std::result::Result<Option<RuntimeItemStack>, LootRuntimeReason>;
}

pub struct LootEvaluator<'a> {
    eval: Eval<'a>,
}

impl<'a> LootEvaluator<'a> {
    pub fn new(
        tables: &'a LootRegistries,
        owner_id: &'a str,
        params: &LootParams<'a>,
        random: &'a GameRandom,
    ) -> Result<Self> {
        Ok(Self {
            eval: Eval::new(owner_id, tables, params, random)?,
        })
    }

    pub fn condition(&mut self, condition: &LootCondition, path: &str) -> Result<bool> {
        self.eval.condition(condition, path)
    }

    #[cfg(test)]
    pub fn function(
        &mut self,
        function: &LootFunction,
        stack: ItemStackSummary,
        path: &str,
    ) -> Result<ItemStackSummary> {
        self.eval.apply_loot_function(function, stack, path)
    }

    pub fn function_runtime(
        &mut self,
        function: &LootFunction,
        stack: RuntimeItemStack,
        path: &str,
    ) -> Result<RuntimeItemStack> {
        self.eval
            .apply_loot_function(function, stack.into_summary(), path)
            .map(RuntimeItemStack::from_summary)
    }

    /// The root function counts as visited while it runs; the result is
    /// clamped to the stack size afterwards.
    pub fn modifier_runtime(
        &mut self,
        function: &LootFunction,
        stack: RuntimeItemStack,
    ) -> Result<RuntimeItemStack> {
        self.eval
            .visited_functions
            .push(self.eval.table_id.to_owned());
        let result = self.function_runtime(function, stack, "modifier");
        self.eval.visited_functions.pop();
        let mut stack = result?;
        let limit = stack.max_stack_size().map_err(|error| {
            self.eval
                .err("modifier", LootRuntimeReason::Catalog(error.to_string()))
        })?;
        stack.set_count(stack.count().min(limit));
        Ok(stack)
    }

    pub fn number_int(&mut self, provider: &NumberProvider, path: &str) -> Result<i32> {
        self.eval.number_int(provider, path)
    }

    pub fn number_float(&mut self, provider: &NumberProvider, path: &str) -> Result<f32> {
        self.eval.number_float(provider, path)
    }

    pub fn random_int(&mut self, bound: i32) -> i32 {
        self.eval.random.next_int(bound)
    }
}

/// `DataComponentPatch.CODEC` over a `set_components` map, decoded against
/// `resolver` on first use.
pub fn prepare_component_patch<'a>(
    components: &'a ComponentPatchParameter,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<&'a DataComponentPatchSummary, LootRuntimeReason> {
    components
        .patch(|value| {
            bbb_registry_codec::read_component_patch(value, resolver)
                .map_err(|error| error.to_string())
        })
        .map_err(catalog_reason)
}

pub(super) fn apply_prepared_component_patch(
    stack: RuntimeItemStack,
    patch: &DataComponentPatchSummary,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> std::result::Result<RuntimeItemStack, LootRuntimeReason> {
    let changed = stack
        .clone()
        .with_component_patch(patch, resolver)
        .map_err(catalog_reason)?;
    // A patch that fails `validateStrict` leaves the old stack.
    let validated =
        bbb_server_datapack::item_stack_template::validate_strict(changed.summary().clone());
    if &validated != changed.summary() {
        return Ok(stack);
    }
    Ok(changed)
}

pub(super) fn catalog_reason(error: impl std::fmt::Display) -> LootRuntimeReason {
    LootRuntimeReason::Catalog(error.to_string())
}

pub fn transmute_runtime_item(
    stack: &RuntimeItemStack,
    item_id: i32,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> std::result::Result<RuntimeItemStack, LootRuntimeReason> {
    crate::inventory::utility_menu::transmute_item_stack(stack, item_id, resolver)
        .map_err(catalog_reason)
}

pub(crate) fn nbt_string(value: &NbtSummaryValue, name: &str) -> Option<String> {
    match nbt_field(value, name)? {
        NbtSummaryValue::String(value) => value.try_to_string().ok(),
        _ => None,
    }
}
