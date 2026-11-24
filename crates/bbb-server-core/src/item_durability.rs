//! Item durability changes and enchantment value effects.

use std::sync::Arc;

use crate::loot_runtime::{
    matches_conditions, param_sets, LootEntities, LootEntity, LootOrigin, LootParams,
    RegistryItemEnvironment,
};
use crate::random::GameRandom;
use crate::world::ValueEffectLevel;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID;
use bbb_protocol::packets::{ItemStackSummary, Vec3d};

use bbb_protocol::equipment_slots::EquipmentSlot;

use crate::inventory::item_damage_per_block;
pub(crate) use bbb_registry_codec::enchantment_value_effect::EnchantmentValueEffect as ValueEffect;
use bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup;
use bbb_server_datapack::loot::LootCondition;
use bbb_server_datapack::startup::DatapackStartup;

/// `EquipmentSlot.MAINHAND`'s entity event (`LivingEntity.java:3770`), which
/// `onEquippedItemBroken` broadcasts (`:3782`) and the client turns into the
/// break sound and particles (`:2056-2058`, `:1412-1424`).
pub const ENTITY_EVENT_MAINHAND_ITEM_BROKEN: i8 = 47;

/// `EquipmentSlot.OFFHAND`'s entity event
/// (`LivingEntity.entityEventForEquipmentBreak`, `LivingEntity.java:3769`).
pub const ENTITY_EVENT_OFFHAND_ITEM_BROKEN: i8 = 48;

/// `EnchantmentEffectComponents.ITEM_DAMAGE`
/// (`EnchantmentEffectComponents.java:57-59`), the component key an
/// enchantment document spells the durability effects under.
pub const ITEM_DAMAGE_COMPONENT: &str = "minecraft:item_damage";

/// `EnchantmentEffectComponents.REPAIR_WITH_XP`
/// (`EnchantmentEffectComponents.java:104-106`), which `minecraft:mending` is
/// the only built-in declarer of.
pub const REPAIR_WITH_XP_COMPONENT: &str = "minecraft:repair_with_xp";

/// `EnchantmentEffectComponents.BLOCK_EXPERIENCE`
/// (`EnchantmentEffectComponents.java:98-100`), which silk touch is the only
/// built-in declarer of.
pub const BLOCK_EXPERIENCE_COMPONENT: &str = "minecraft:block_experience";

/// `EnchantmentEffectComponents.FISHING_TIME_REDUCTION` and
/// `FISHING_LUCK_BONUS` (`EnchantmentEffectComponents.java:91-98`): lure and
/// luck of the sea.
pub const FISHING_TIME_REDUCTION_COMPONENT: &str = "minecraft:fishing_time_reduction";
pub const FISHING_LUCK_BONUS_COMPONENT: &str = "minecraft:fishing_luck_bonus";

/// `EnchantmentEffectComponents.MOB_EXPERIENCE` (`EnchantmentEffectComponents.java:101-103`).
pub const MOB_EXPERIENCE_COMPONENT: &str = "minecraft:mob_experience";

/// What one `ItemStack.mineBlock` did (`ItemStack.java:557-562`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MineBlockOutcome {
    /// `Item.mineBlock`'s return value (`Item.java:260`, `:267`): false for a
    /// stack with no `DataComponents.TOOL`, which is what gates the
    /// `Stats.ITEM_USED` award at `ItemStack.java:560`.
    pub used: bool,
    /// The item the break consumed, when `applyDamage` shrank the stack to
    /// nothing (`ItemStack.java:468-472`). It is the id the
    /// `Stats.ITEM_BROKEN` award keys on (`ServerPlayer.java:2202`).
    pub broken_item_id: Option<i32>,
    /// `applyDamage` input before clamping or consuming the stack.
    pub raw_new_damage: Option<i32>,
}

/// One enchantment's list of `ConditionalEffect<EnchantmentValueEffect>` for
/// one value component (`EnchantmentEffectComponents.java:57-110`).
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantmentValueEffects {
    entries: Vec<ConditionalValueEffect>,
    slots: Vec<String>,
    generation: Option<ValueEffectGeneration>,
    /// Why the document's declaration of the component could not be turned
    /// into effects, for an enchantment [`Self::parse`] refused.
    ///
    /// A refusal is a table entry rather than an absence, because the two mean
    /// opposite things at the point the value is computed: an absence is "this
    /// enchantment does not touch the value" and a stack carrying it wears out
    /// at exactly vanilla's rate, while a refusal is "this enchantment touches
    /// it in a way this server cannot compute" and the number that comes out
    /// is wrong. Dropping the refusal to `None` -- which is what this loader
    /// did -- turns the second into the first and takes the evidence with it.
    unmodelled: Option<String>,
}

/// `EquipmentSlotGroup.test` (`EquipmentSlotGroup.java:14-25`) by serialized
/// name, which is what an enchantment document's `slots` list holds.
///
/// An unknown group name answers false: the codec would have refused the
/// document outright, and matching everywhere would put an enchantment in
/// slots vanilla never puts it in.
#[must_use]
pub fn equipment_slot_group_matches(group: &str, slot: EquipmentSlot) -> bool {
    match group {
        "any" => true,
        "mainhand" => slot == EquipmentSlot::MainHand,
        "offhand" => slot == EquipmentSlot::OffHand,
        // `EquipmentSlot.Type.HAND` (`EquipmentSlot.java:13-14`).
        "hand" => matches!(slot, EquipmentSlot::MainHand | EquipmentSlot::OffHand),
        "feet" => slot == EquipmentSlot::Feet,
        "legs" => slot == EquipmentSlot::Legs,
        "chest" => slot == EquipmentSlot::Chest,
        "head" => slot == EquipmentSlot::Head,
        // `EquipmentSlot.isArmor()` includes animal body armor, not saddles.
        "armor" => matches!(
            slot,
            EquipmentSlot::Feet
                | EquipmentSlot::Legs
                | EquipmentSlot::Chest
                | EquipmentSlot::Head
                | EquipmentSlot::Body
        ),
        "body" => slot == EquipmentSlot::Body,
        "saddle" => slot == EquipmentSlot::Saddle,
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ConditionalValueEffect {
    /// `ConditionalEffect.requirements` (`ConditionalEffect.java:22-24`), an
    /// empty requirement always matching.
    requirement: Option<LootCondition>,
    effect: ValueEffect,
}

/// The datapack generation requirements are evaluated against: its loot
/// registries, live tags and enchantment ids.
#[derive(Clone)]
pub struct ValueEffectGeneration {
    enchantment: Arc<str>,
    startup: Arc<DatapackStartup>,
    registry: Arc<Vec<String>>,
}

impl ValueEffectGeneration {
    /// The loaded generation, or the built-in pack without one.
    #[must_use]
    pub fn new(datapacks: Option<&Arc<DatapackStartup>>) -> Self {
        let startup = datapacks
            .cloned()
            .unwrap_or_else(crate::registry_sync::builtin_datapack_startup_shared);
        let registry = startup
            .registries
            .registry(crate::enchantment_facts::ENCHANTMENT_REGISTRY)
            .map(|registry| registry.elements.iter().map(|e| e.id.clone()).collect())
            .unwrap_or_default();
        Self {
            enchantment: Arc::from(""),
            startup,
            registry: Arc::new(registry),
        }
    }

    fn named(&self, enchantment: &str) -> Self {
        Self {
            enchantment: Arc::from(enchantment),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn lookup(&self) -> FrozenDatapackRegistryLookup<'_> {
        FrozenDatapackRegistryLookup::from_startup(&self.startup)
    }
}

impl std::fmt::Debug for ValueEffectGeneration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.enchantment)
    }
}

impl PartialEq for ValueEffectGeneration {
    fn eq(&self, other: &Self) -> bool {
        self.enchantment == other.enchantment && Arc::ptr_eq(&self.startup, &other.startup)
    }
}

/// The loot context a value component's requirements are tested in.
#[derive(Clone, Copy)]
pub enum ValueContext<'a> {
    /// `Enchantment.itemContext` (`Enchantment.java:475-481`): `ENCHANTED_ITEM`.
    Item(&'a ItemStackSummary),
    /// `Enchantment.entityContext` (`Enchantment.java:493-500`): `ENCHANTED_ENTITY`.
    Entity { this: LootEntity<'a>, origin: Vec3d },
}

impl<'a> ValueContext<'a> {
    /// The entity context of a stored entity at its own position.
    #[must_use]
    pub fn stored(
        entity: &'a crate::entity::Entity,
        bodies: Option<crate::entity::store::EntityBodyReadView<'a>>,
    ) -> Self {
        Self::Entity {
            this: LootEntity::Stored(crate::loot_runtime::StoredLootEntity {
                entity,
                bodies,
                raid_at_position: None,
            }),
            origin: entity.position(),
        }
    }
}

impl EnchantmentValueEffects {
    /// The `effects.<component>` member of one enchantment document, or `None`
    /// when it declares none.
    ///
    /// `Err` is a shape this crate does not understand, which the caller
    /// records: it is the difference between "this enchantment does not touch
    /// the value" and "this enchantment touches it in a way this server cannot
    /// compute".
    pub fn parse(
        document: &bbb_registry_codec::RegistryJsonValue,
        component: &str,
    ) -> Result<Option<Self>, String> {
        Self::parse_in(&ValueEffectGeneration::new(None), "", document, component)
    }

    /// [`Self::parse`] against one loaded generation, for enchantment `id`.
    pub fn parse_in(
        generation: &ValueEffectGeneration,
        id: &str,
        document: &bbb_registry_codec::RegistryJsonValue,
        component: &str,
    ) -> Result<Option<Self>, String> {
        let Some(list) = crate::enchantment_facts::effect_component(document, component) else {
            return Ok(None);
        };
        let lookup = generation.lookup();
        // A bare `EnchantmentValueEffect` is the unconditional shape of
        // components such as `crossbow_charge_time`.
        let single;
        let list = match list {
            bbb_registry_codec::RegistryJsonValue::Object(_) => {
                single = [bbb_registry_codec::RegistryJsonValue::Object(vec![(
                    "effect".into(),
                    list.clone(),
                )])];
                &single[..]
            }
            _ => list
                .as_array()
                .ok_or_else(|| format!("{component} is not a list"))?,
        };
        // `Enchantment.definition.slots` (`Enchantment.java:126-128`), the
        // groups `matchingSlot` tests. An enchantment with no `slots` at all is
        // not one any vanilla codec built, and is refused rather than matched
        // everywhere.
        let slots = document
            .get("slots")
            .and_then(bbb_registry_codec::RegistryJsonValue::as_array)
            .map(|groups| {
                groups
                    .iter()
                    .filter_map(bbb_registry_codec::RegistryJsonValue::utf8_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut entries = Vec::with_capacity(list.len());
        for conditional in list {
            let effect = conditional
                .get("effect")
                .ok_or_else(|| format!("a conditional {component} effect has no effect"))?;
            let effect =
                ValueEffect::parse(effect).map_err(|detail| format!("{component}: {detail}"))?;
            let requirement = conditional
                .get("requirements")
                .map(|value| {
                    LootCondition::decode(
                        bbb_registry_codec::RegistryElementValueRef::Json(value),
                        Some(&lookup),
                    )
                })
                .transpose()
                .map_err(|error| format!("{component} requirements: {error}"))?;
            entries.push(ConditionalValueEffect {
                requirement,
                effect,
            });
        }
        Ok(Some(Self {
            entries,
            slots,
            generation: Some(generation.named(id)),
            unmodelled: None,
        }))
    }

    /// The table entry for an enchantment whose declaration of the component
    /// [`Self::parse`] refused.
    ///
    /// It carries no effects and no slots: the shape they would have come from
    /// is the shape that could not be read. What it does carry is the reason,
    /// which every consumer of the value reports in place of a number it
    /// cannot compute.
    #[must_use]
    pub fn unmodelled(detail: String) -> Self {
        Self {
            entries: Vec::new(),
            slots: Vec::new(),
            generation: None,
            unmodelled: Some(detail),
        }
    }

    /// `Enchantment.modifyItemFilteredCount` / `modifyEntityFilteredValue`
    /// (`Enchantment.java:415-444`) for one enchantment: every effect whose
    /// requirements match in `context` processes the running value in list
    /// order (`Enchantment.applyEffects`, `:514-526`).
    pub(crate) fn apply(
        &self,
        level: i32,
        context: ValueContext<'_>,
        value: f32,
        random: &GameRandom,
        value_level: &ValueEffectLevel<'_>,
    ) -> Result<f32, String> {
        if let Some(detail) = &self.unmodelled {
            return Err(detail.clone());
        }
        let mut random_ref = random;
        let mut effect_random = Some(&mut random_ref);
        if self.entries.iter().all(|entry| entry.requirement.is_none()) {
            return Ok(self.entries.iter().fold(value, |value, entry| {
                entry.effect.process(level, value, &mut effect_random)
            }));
        }
        self.with_requirements(level, context, random, value_level, |matches| {
            let mut value = value;
            for entry in &self.entries {
                if let Some(condition) = &entry.requirement {
                    if !matches(condition)? {
                        continue;
                    }
                }
                value = entry.effect.process(level, value, &mut effect_random);
            }
            Ok(value)
        })
    }

    /// `body` with a requirement test bound to `context`.
    fn with_requirements<R>(
        &self,
        level: i32,
        context: ValueContext<'_>,
        random: &GameRandom,
        value_level: &ValueEffectLevel<'_>,
        body: impl FnOnce(&mut dyn FnMut(&LootCondition) -> Result<bool, String>) -> R,
    ) -> R {
        let Some(generation) = self.generation.as_ref() else {
            return body(&mut |_| Err("a value effect without its generation".into()));
        };
        let lookup = generation.lookup();
        let environment = RegistryItemEnvironment::for_enchantment(&lookup, value_level);
        let tool;
        let params = match context {
            ValueContext::Item(stack) => {
                tool = crate::entity::block_drops::loot_tool_for_stack(stack, &generation.registry);
                LootParams {
                    tool: Some(&tool),
                    enchantment_level: Some(level),
                    item_environment: Some(&environment),
                    weather: Some(value_level.weather),
                    ..LootParams::new(param_sets::ENCHANTED_ITEM)
                }
            }
            ValueContext::Entity { this, origin } => LootParams {
                origin: Some(LootOrigin {
                    x: origin.x,
                    y: origin.y,
                    z: origin.z,
                }),
                entities: LootEntities {
                    this_entity: this,
                    ..LootEntities::default()
                },
                enchantment_level: Some(level),
                item_environment: Some(&environment),
                weather: Some(value_level.weather),
                ..LootParams::new(param_sets::ENCHANTED_ENTITY)
            },
        };
        body(&mut |condition| {
            matches_conditions(
                &generation.startup.loot,
                &generation.enchantment,
                std::slice::from_ref(condition),
                &params,
                random,
            )
            .map_err(|error| error.to_string())
        })
    }
}

impl EnchantmentValueEffects {
    /// `Enchantment.matchingSlot(EquipmentSlot)` (`Enchantment.java:126-128`).
    ///
    /// An enchantment this crate refused answers true for every slot. Its own
    /// `slots` list is part of what could not be read, so the honest answer is
    /// not "no slot" -- that would silently drop the enchantment out of every
    /// walk that selects on it, and the refusal with it. Answering everywhere
    /// puts it in front of [`Self::apply`], which is where the refusal is
    /// recorded.
    #[must_use]
    pub fn matching_slot(&self, slot: EquipmentSlot) -> bool {
        self.unmodelled.is_some()
            || self
                .slots
                .iter()
                .any(|group| equipment_slot_group_matches(group, slot))
    }
}

/// The enchantments on one stack that touch durability: the level, and the
/// effects the loaded registry declares for them.
///
/// `Err` names an enchantment whose effects this crate cannot compute, which
/// the caller records rather than dropping -- see [`EnchantmentValueEffects`].
pub type ValueEffectEnchantments<'a> = &'a [(i32, &'a EnchantmentValueEffects)];

/// `ItemStack.mineBlock(Level, BlockState, BlockPos, Player)`
/// (`ItemStack.java:557-562`) over `Item.mineBlock` (`Item.java:257-268`).
///
/// `destroy_speed` is `state.getDestroySpeed(level, pos)` (`Item.java:263`),
/// which the generated block-state destroy facts already answer. `in_fire_tag`
/// is `state.is(BlockTags.FIRE)`, the guard `ShearsItem.mineBlock`
/// (`ShearsItem.java:48-59`) wears on instead; `minecraft:shears` is the only
/// `ShearsItem` (`Items.java:1491`).
/// `has_infinite_materials` is `Player.hasInfiniteMaterials`
/// (`Player.java:1293-1295`), the creative bit `processDurabilityChange` refuses
/// on (`ItemStack.java:455-456`).
///
/// The stack is written in place, which is what vanilla does: the stack
/// `ServerPlayerGameMode.destroyBlock` hands over is the *live* main-hand one
/// (`ServerPlayerGameMode.java:293`, `:296`), and the copy it took a line
/// earlier is what the loot table later sees.
pub fn mine_block(
    stack: &mut ItemStackSummary,
    destroy_speed: f32,
    in_fire_tag: bool,
    has_infinite_materials: bool,
    enchantments: ValueEffectEnchantments<'_>,
    random: &GameRandom,
    value_level: &ValueEffectLevel<'_>,
) -> MineBlockOutcome {
    let mut outcome = MineBlockOutcome::default();
    // `Item.java:258-261`.
    let Some(damage_per_block) = damage_per_block(stack) else {
        return outcome;
    };
    outcome.used = true;
    // `Item.java:263-265`. `level.isClientSide()` is always false here.
    let wears = if stack.item_id == Some(bbb_protocol::item_types::VANILLA_ITEM_SHEARS_ID) {
        !in_fire_tag
    } else {
        destroy_speed != 0.0
    };
    if wears && damage_per_block > 0 {
        hurt_and_break(
            stack,
            damage_per_block,
            has_infinite_materials,
            enchantments,
            random,
            value_level,
            &mut outcome,
        );
    }
    outcome
}

/// `ItemStack.hurtAndBreak(int, ServerLevel, ServerPlayer, Consumer<Item>)`
/// (`ItemStack.java:445-450`) over `processDurabilityChange` (`:452-460`) and
/// `applyDamage` (`:462-473`).
///
/// `pub(crate)` for the non-mining callers `hurtAndBreak(1, ..)`, such as
/// `TntBlock.useItemOn`'s flint and steel (`TntBlock.java:120`).
pub fn hurt_and_break(
    stack: &mut ItemStackSummary,
    amount: i32,
    has_infinite_materials: bool,
    enchantments: ValueEffectEnchantments<'_>,
    random: &GameRandom,
    value_level: &ValueEffectLevel<'_>,
    outcome: &mut MineBlockOutcome,
) {
    let Some(max_damage) = damageable_max_damage(stack) else {
        // `:453-454`: not a damageable item, so the change is zero and
        // `applyDamage` never runs.
        return;
    };
    // `:455-456`.
    if has_infinite_materials {
        return;
    }
    // `:458`. Nonpositive amounts pass through without enchantment processing.
    let amount = if amount > 0 {
        process_durability_change(stack, amount, enchantments, random, value_level)
    } else {
        amount
    };
    // `:447`.
    if amount == 0 {
        return;
    }
    // `:448` over `applyDamage` (`:462-473`), whose `setDamageValue` clamps
    // into `0..=maxDamage` (`:429-431`).
    let raw_new_damage = current_damage(stack, max_damage).wrapping_add(amount);
    outcome.raw_new_damage = Some(raw_new_damage);
    let new_damage = raw_new_damage.clamp(0, max_damage.max(0));
    set_damage(stack, new_damage);
    // `:468-472` over `isBroken` (`:437-439`).
    if new_damage >= max_damage {
        let item_id = stack.item_id;
        // `ItemStack.shrink(1)` (`:1078-1080`) through `setCount`, which for a
        // one-item tool empties the stack. Vanilla leaves the object in the
        // slot with a zero count and `isEmpty()` (`:320-322`) speaks for it,
        // where this crate spells an empty slot out -- the same normalisation
        // `Inventory.consume_in_hand` already applies, and the same thing the
        // wire codec writes either way.
        stack.count -= 1;
        if stack.count <= 0 {
            *stack = ItemStackSummary::empty();
        }
        outcome.broken_item_id = item_id;
    }
}

/// `ItemStack.hurtWithoutBreaking` (`ItemStack.java:475-485`): worn down
/// to one use short of breaking at most.
pub fn hurt_without_breaking(
    stack: &mut ItemStackSummary,
    amount: i32,
    has_infinite_materials: bool,
    enchantments: ValueEffectEnchantments<'_>,
    random: &GameRandom,
    value_level: &ValueEffectLevel<'_>,
) -> Option<i32> {
    let max_damage = damageable_max_damage(stack)?;
    if has_infinite_materials {
        return None;
    }
    let amount = if amount > 0 {
        process_durability_change(stack, amount, enchantments, random, value_level)
    } else {
        amount
    };
    if amount == 0 {
        return None;
    }
    let damage = current_damage(stack, max_damage)
        .wrapping_add(amount)
        .min(max_damage - 1);
    set_damage(stack, damage.max(0));
    Some(damage)
}

/// `ItemStack.nextDamageWillBreak` (`ItemStack.java:441-443`).
pub fn next_damage_will_break(stack: &ItemStackSummary) -> bool {
    damageable_max_damage(stack)
        .is_some_and(|max_damage| current_damage(stack, max_damage) >= max_damage - 1)
}

/// `EnchantmentHelper.processDurabilityChange`
/// (`EnchantmentHelper.java:90-94`) over `Enchantment.modifyDurabilityChange`
/// (`Enchantment.java:222-224`).
///
/// The running value is a `MutableFloat` seeded with the amount and narrowed by
/// `intValue()` at the end, which truncates toward zero.
fn process_durability_change(
    stack: &ItemStackSummary,
    amount: i32,
    enchantments: ValueEffectEnchantments<'_>,
    random: &GameRandom,
    value_level: &ValueEffectLevel<'_>,
) -> i32 {
    process_value_effects(stack, amount as f32, enchantments, random, value_level) as i32
}

/// The body `EnchantmentHelper.processDurabilityChange`
/// (`EnchantmentHelper.java:90-94`) and `processBlockExperience` (`:102-106`)
/// share: a `MutableFloat` seeded with the amount, `runIterationOnItem`
/// (`:126-132`) over the stack's enchantments, and the running value back out.
///
/// The two callers differ only in the component they resolved the effects from
/// and in where they narrow the result, so this is one function rather than
/// two: the arithmetic and the draw order are the same, and a second copy would
/// be a second place for them to drift.
///
/// An effect that cannot be computed leaves the running value unchanged.
pub fn process_value_effects(
    stack: &ItemStackSummary,
    value: f32,
    enchantments: ValueEffectEnchantments<'_>,
    random: &GameRandom,
    value_level: &ValueEffectLevel<'_>,
) -> f32 {
    if stack.item_id.is_none() {
        return value;
    }
    process_value_effects_in(
        ValueContext::Item(stack),
        value,
        enchantments,
        random,
        value_level,
    )
}

/// [`process_value_effects`] for a component read in `context`.
pub fn process_value_effects_in(
    context: ValueContext<'_>,
    value: f32,
    enchantments: ValueEffectEnchantments<'_>,
    random: &GameRandom,
    value_level: &ValueEffectLevel<'_>,
) -> f32 {
    let mut value = value;
    for (level, effects) in enchantments {
        match effects.apply(*level, context, value, random, value_level) {
            Ok(next) => value = next,
            Err(detail) => tracing::debug!("enchantment value effect: {detail}"),
        }
    }
    value
}

/// `Tool.damagePerBlock` (`Tool.java:17`) for the stack's effective
/// `DataComponents.TOOL`, or `None` for a stack that carries none -- which is
/// `Item.mineBlock`'s `return false` (`Item.java:259-261`).
fn damage_per_block(stack: &ItemStackSummary) -> Option<i32> {
    item_damage_per_block(stack).ok().flatten()
}

/// `ItemStack.isDamageableItem()` (`ItemStack.java:417-419`) answered together
/// with the `getMaxDamage` (`:433-435`) it needs: `MAX_DAMAGE` present, no
/// `UNBREAKABLE`, and `DAMAGE` present.
fn damageable_max_damage(stack: &ItemStackSummary) -> Option<i32> {
    bbb_server_types::item_stack::rules::damageable_max_damage(stack)
}

/// `ItemStack.isDamaged()` (`ItemStack.java:421-423`) together with the two
/// numbers a repair needs: the current damage and the maximum.
#[must_use]
pub fn damage_and_max(stack: &ItemStackSummary) -> Option<(i32, i32)> {
    bbb_server_types::item_stack::rules::damage_and_max(stack)
}

/// `ItemStack.setDamageValue(int)` for a caller outside this module, with
/// `applyDamage`'s own clamp already applied.
pub fn repair_to(stack: &mut ItemStackSummary, damage: i32) {
    let Some((_, max_damage)) = damage_and_max(stack) else {
        return;
    };
    set_damage(stack, damage.clamp(0, max_damage.max(0)));
}

/// `ItemStack.getDamageValue()` (`ItemStack.java:425-427`): the `DAMAGE`
/// component clamped into range, with `0` for a stack that somehow lost it.
fn current_damage(stack: &ItemStackSummary, max_damage: i32) -> i32 {
    bbb_server_types::item_stack::rules::current_damage(stack, max_damage)
}

/// `ItemStack.setDamageValue(int)` (`ItemStack.java:429-431`), which is a patch
/// write: the prototype's own `DAMAGE` stays where it is and the stack carries
/// the override.
fn set_damage(stack: &mut ItemStackSummary, damage: i32) {
    stack.component_patch.damage = Some(damage);
    stack
        .component_patch
        .removed_type_ids
        .retain(|id| *id != VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    if !stack
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)
    {
        stack
            .component_patch
            .added_type_ids
            .push(VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID);
    }
}

#[cfg(test)]
mod tests;
