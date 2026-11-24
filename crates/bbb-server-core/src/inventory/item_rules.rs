//! What an item stack allows: use branch, attack range, mining tools, adventure-mode
//! predicates and stack sizes.

use std::collections::BTreeMap;

use bbb_protocol::{
    attribute_modifier_types::{
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID,
        VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID,
    },
    attribute_types::{
        vanilla_attribute_type, VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
        VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
    },
    block_state_types::{
        vanilla_block_property_kind, vanilla_block_property_value_rank,
        vanilla_block_state_block_type_id, vanilla_block_state_identity, VanillaBlockStateIdentity,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
        VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID, VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID,
        VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID,
    },
    equipment_slots::{
        VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID, VANILLA_EQUIPMENT_SLOT_GROUP_HAND_ID,
        VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID,
    },
    item_types::VANILLA_DESTROY_BLOCK_DENIED_ITEM_IDS,
    packets::{
        AttributeModifierSummary, ItemStackSummary, NbtSummaryValue, StatePropertyMatcherSummary,
        StatePropertyValueMatcherSummary, ToolRuleSummary,
    },
};

use crate::registry_tags::RegistryTagSnapshot;
use bbb_server_types::block_state_parse::block_entity_data_matches;
use bbb_server_types::interaction::AttackRange;
use bbb_server_types::item_stack::static_item_component;
use bbb_server_types::{Result, ServerWorldError};

/// Whether a stack carries a component at all, resolved the way
/// `DataComponentGetter.has` sees it: the patch adds it, the patch removes it,
/// or the item prototype declares it.
/// `PatchedDataComponentMap.set` for a component whose prototype value the new
/// one differs from: the patch gains the entry, and gains it once.
pub fn add_patched_component(item: &mut ItemStackSummary, type_id: i32) {
    bbb_server_types::item_stack::rules::add_patched_component(item, type_id)
}

/// `PatchedDataComponentMap.remove` for a component the new prototype does not
/// carry: the patch entry is dropped outright rather than marked removed.
pub fn remove_patched_component(item: &mut ItemStackSummary, type_id: i32) {
    bbb_server_types::item_stack::rules::remove_patched_component(item, type_id)
}

pub fn has_component(item: &ItemStackSummary, type_id: i32) -> Result<bool> {
    bbb_server_types::item_stack::rules::has_component(item, type_id)
}

/// `DataComponentGetter.getOrDefault` for an integer component the patch
/// summary already projects.
fn component_i32(
    item: &ItemStackSummary,
    type_id: i32,
    patched: Option<i32>,
    absent: i32,
) -> Result<i32> {
    bbb_server_types::item_stack::rules::component_i32(item, type_id, patched, absent)
}

pub(super) fn prototype_components(
    item_id: i32,
) -> Result<Option<&'static std::collections::BTreeMap<i32, serde_json::Value>>> {
    bbb_server_types::item_stack::rules::prototype_components(item_id)
}

/// `LivingEntity.getAttackRangeWith` (LivingEntity.java:2193-2196): the reach
/// band the weapon declares, or `None` when it declares none and the attacker's
/// own default applies.
pub fn attack_range(item: &ItemStackSummary) -> Result<Option<AttackRange>> {
    if item
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID)
    {
        return Ok(item.component_patch.attack_range.map(|range| AttackRange {
            min_reach: range.min_reach,
            max_reach: range.max_reach,
            min_creative_reach: range.min_creative_reach,
            max_creative_reach: range.max_creative_reach,
            hitbox_margin: range.hitbox_margin,
            mob_factor: range.mob_factor,
        }));
    }
    let Some(value) =
        prototype_component_object(item, VANILLA_DATA_COMPONENT_ATTACK_RANGE_TYPE_ID)?
    else {
        return Ok(None);
    };
    // `AttackRange.CODEC` (AttackRange.java:28-38) makes every field optional,
    // and the generated catalog writes the resolved record, so a field that is
    // missing here is a catalog fault rather than a codec default.
    let field = |name: &str| -> Result<f32> {
        value
            .get(name)
            .and_then(serde_json::Value::as_f64)
            .map(|number| number as f32)
            .ok_or(ServerWorldError::MissingCatalogFact("attack range field"))
    };
    Ok(Some(AttackRange {
        min_reach: field("min_reach")?,
        max_reach: field("max_reach")?,
        min_creative_reach: field("min_creative_reach")?,
        max_creative_reach: field("max_creative_reach")?,
        hitbox_margin: field("hitbox_margin")?,
        mob_factor: field("mob_factor")?,
    }))
}

/// `ItemStack.canDestroyBlock` (ItemStack.java:1122-1124) for the one caller
/// this server has, `ServerPlayerGameMode.destroyBlock` (:264-266), whose user
/// is always the breaking `Player`.
///
/// Two different things refuse, and neither is a hand-written list. The first
/// is an `Item.canDestroyBlock` override that answers false whatever the stack
/// holds, which in 26.1 is `DebugStickItem.canDestroyBlock`
/// (DebugStickItem.java:29-35) alone;
/// [`VANILLA_DESTROY_BLOCK_DENIED_ITEM_IDS`] is that set, extracted from the
/// vanilla items rather than typed out. The second is the base
/// `Item.canDestroyBlock` (Item.java:169-172): a `DataComponents.TOOL` whose
/// `canDestroyBlocksInCreative` is false refuses every block for a user with
/// `instabuild`, which in 26.1 is every sword (`ToolMaterial.applySwordProperties`
/// builds `new Tool(.., 1.0F, 2, false)`, ToolMaterial.java:82-91), the trident
/// (TridentItem.java:47-49) and the mace (MaceItem.java:46-48) -- all of them
/// read out of the prototype catalog here, none of them named.
pub fn can_destroy_block(item: &ItemStackSummary, instabuild: bool) -> Result<bool> {
    if item.count > 0
        && item
            .item_id
            .is_some_and(|item_id| VANILLA_DESTROY_BLOCK_DENIED_ITEM_IDS.contains(&item_id))
    {
        return Ok(false);
    }
    if !instabuild {
        return Ok(true);
    }
    can_destroy_blocks_in_creative(item)
}

/// `ItemStack.canPlaceOnBlockInAdventureMode` / `canBreakBlockInAdventureMode`
/// (ItemStack.java:1037-1045) through `AdventureModePredicate.test`
/// (AdventureModePredicate.java:77-94): a stack without the component fails;
/// otherwise any `BlockPredicate.matches(BlockInWorld)` (BlockPredicate.java:73-81)
/// admits the block. `tag_contains(tag, block_type_id)` reads the level's tags;
/// `block_entity_data` is `saveWithFullMetadata` of the block entity there.
pub fn adventure_mode_predicate_matches(
    item: &ItemStackSummary,
    can_break: bool,
    state: i32,
    tag_contains: impl Fn(&str, i32) -> bool,
    block_entity_data: impl Fn() -> Option<NbtSummaryValue>,
) -> bool {
    adventure_mode_predicate_matches_with_registry(
        item,
        can_break,
        state,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
        tag_contains,
        block_entity_data,
    )
}

pub fn adventure_mode_predicate_matches_with_registry(
    item: &ItemStackSummary,
    can_break: bool,
    state: i32,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
    tag_contains: impl Fn(&str, i32) -> bool,
    block_entity_data: impl Fn() -> Option<NbtSummaryValue>,
) -> bool {
    let patch = &item.component_patch;
    let (type_id, predicate) = if can_break {
        (
            VANILLA_DATA_COMPONENT_CAN_BREAK_TYPE_ID,
            patch.can_break_persistent.as_ref(),
        )
    } else {
        (
            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
            patch.can_place_on_persistent.as_ref(),
        )
    };
    if item.count <= 0 || !patch.added_type_ids.contains(&type_id) {
        return false;
    }
    let Some(predicate) = predicate else {
        return false;
    };
    let vanilla_identity = vanilla_block_state_identity(state);
    let custom_block = if vanilla_identity.is_none() {
        content_registry.block_by_state_id(state)
    } else {
        None
    };
    let Some(block) = vanilla_block_state_block_type_id(state)
        .or_else(|| custom_block.map(|(block, _)| block.block_id))
    else {
        return false;
    };
    predicate.predicates.iter().any(|predicate| {
        let blocks_match = predicate.blocks.as_ref().is_none_or(|blocks| {
            match (blocks.registry_ids.as_slice(), blocks.tag.as_deref()) {
                (ids, None) => ids.contains(&block),
                ([], Some(tag)) => tag_contains(tag, block),
                _ => false,
            }
        });
        let state_match = predicate.state.as_ref().is_none_or(|matchers| {
            if let Some(identity) = vanilla_identity {
                matchers
                    .iter()
                    .all(|matcher| state_matcher_matches(matcher, identity))
            } else if let Some((_, offset)) = custom_block {
                let offset_str = offset.to_string();
                let value = i32::from(offset);
                matchers.iter().all(|matcher| {
                    matcher.name == bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY
                        && match &matcher.value {
                            bbb_protocol::packets::StatePropertyValueMatcherSummary::Exact(
                                expected,
                            ) => *expected == offset_str,
                            bbb_protocol::packets::StatePropertyValueMatcherSummary::Range {
                                min,
                                max,
                            } => {
                                min.as_ref().is_none_or(|min| {
                                    min.try_to_string()
                                        .ok()
                                        .and_then(|s| s.parse::<i32>().ok())
                                        .is_some_and(|min| value >= min)
                                }) && max.as_ref().is_none_or(|max| {
                                    max.try_to_string()
                                        .ok()
                                        .and_then(|s| s.parse::<i32>().ok())
                                        .is_some_and(|max| value <= max)
                                })
                            }
                        }
                })
            } else {
                false
            }
        });
        blocks_match
            && state_match
            && predicate.nbt.as_ref().is_none_or(|expected| {
                block_entity_data_matches(Some(expected), block_entity_data().as_ref())
            })
    })
}

/// `StatePropertiesPredicate.PropertyMatcher.match` (StatePropertiesPredicate.java):
/// values compare by their parsed rank; a missing property or bad bound fails.
fn state_matcher_matches(
    matcher: &StatePropertyMatcherSummary,
    identity: &VanillaBlockStateIdentity,
) -> bool {
    let Ok(name) = String::from_utf16(matcher.name.units()) else {
        return false;
    };
    let (Some(actual), Some(kind)) = (
        identity.properties.get(name.as_str()),
        vanilla_block_property_kind(&identity.name, &name),
    ) else {
        return false;
    };
    let rank = |value: &bbb_protocol::JavaString| {
        String::from_utf16(value.units())
            .ok()
            .and_then(|value| vanilla_block_property_value_rank(kind, &value))
    };
    let Some(actual) = vanilla_block_property_value_rank(kind, actual) else {
        return false;
    };
    match &matcher.value {
        StatePropertyValueMatcherSummary::Exact(expected) => rank(expected) == Some(actual),
        StatePropertyValueMatcherSummary::Range { min, max } => {
            min.as_ref()
                .is_none_or(|min| rank(min).is_some_and(|min| actual >= min))
                && max
                    .as_ref()
                    .is_none_or(|max| rank(max).is_some_and(|max| actual <= max))
        }
    }
}

/// `ItemStack.getDestroySpeed` (ItemStack.java:374-376) reaching
/// `Item.getDestroySpeed` (Item.java:183-186): the held tool's mining speed for
/// this block, or `1.0` when the stack carries no `DataComponents.TOOL`.
pub fn item_destroy_speed(
    item: &ItemStackSummary,
    block_type_id: i32,
    tags: &RegistryTagSnapshot,
) -> Result<f32> {
    let tool = static_item_component(item, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)?
        .and_then(|component| component.tool.as_ref());
    Ok(match tool {
        Some(tool) => tool
            .rules
            .iter()
            .find_map(|rule| {
                rule.speed_bits
                    .filter(|_| tool_rule_matches(rule, block_type_id, tags))
            })
            .map(f32::from_bits)
            .unwrap_or_else(|| f32::from_bits(tool.default_mining_speed_bits)),
        None => 1.0,
    })
}

/// `ItemStack.isCorrectToolForDrops` (ItemStack.java:564-566) reaching
/// `Item.isCorrectToolForDrops` (Item.java:270-273): `tool != null &&
/// tool.isCorrectForDrops(state)`, so a stack without a tool is never correct.
pub fn item_is_correct_tool_for_drops(
    item: &ItemStackSummary,
    block_type_id: i32,
    tags: &RegistryTagSnapshot,
) -> Result<bool> {
    let tool = static_item_component(item, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)?
        .and_then(|component| component.tool.as_ref());
    Ok(tool.is_some_and(|tool| {
        tool.rules
            .iter()
            .find_map(|rule| {
                rule.correct_for_drops
                    .filter(|_| tool_rule_matches(rule, block_type_id, tags))
            })
            .unwrap_or(false)
    }))
}

/// `Tool.damagePerBlock` (Tool.java:17) for the stack's effective
/// `DataComponents.TOOL`, or `None` for a stack that carries none -- which is
/// `Item.mineBlock`'s early `return false` (Item.java:259-261).
pub fn item_damage_per_block(item: &ItemStackSummary) -> Result<Option<i32>> {
    Ok(
        static_item_component(item, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)?
            .and_then(|component| component.tool.as_ref())
            .map(|tool| tool.damage_per_block),
    )
}

/// One integer-valued component of an item's *prototype*
/// (`Item.components()`, Item.java:155-157), as the generated catalog carries
/// it in codec form.
///
/// `DataComponents.MAX_DAMAGE` and `DataComponents.DAMAGE` are the two this
/// server reads: both are plain ints that `Item.Properties.durability`
/// (Item.java:406-411) attaches together. `None` is a prototype that does not
/// carry the component at all, which for `MAX_DAMAGE` is what makes an item
/// undamageable (`ItemStack.isDamageableItem`, ItemStack.java:417-419).
pub fn item_prototype_int_component(item_id: i32, type_id: i32) -> Option<i32> {
    bbb_server_types::item_stack::rules::item_prototype_int_component(item_id, type_id)
}

fn tool_rule_matches(
    rule: &ToolRuleSummary,
    block_type_id: i32,
    tags: &RegistryTagSnapshot,
) -> bool {
    match &rule.block_tag {
        Some(tag_id) => tags.contains_block(tag_id, block_type_id),
        None => rule.block_registry_ids.contains(&block_type_id),
    }
}

pub fn can_destroy_blocks_in_creative(item: &ItemStackSummary) -> Result<bool> {
    if let Some(tool) = &item.component_patch.tool {
        return Ok(tool.can_destroy_blocks_in_creative);
    }
    Ok(
        static_item_component(item, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID)?
            .and_then(|component| component.tool.as_ref())
            .is_none_or(|tool| tool.can_destroy_blocks_in_creative),
    )
}

/// Whether the stack carries `DataComponents.PIERCING_WEAPON`, which
/// `handleAttack` (:1791) uses to route the swing away from the melee path.
pub fn has_piercing_weapon(item: &ItemStackSummary) -> Result<bool> {
    has_component(item, VANILLA_DATA_COMPONENT_PIERCING_WEAPON_TYPE_ID)
}

/// `DataComponents.WEAPON`'s `itemDamagePerAttack` (`Weapon.java:14`), or
/// `None` for a stack without the component.
pub fn weapon_item_damage_per_attack(item: &ItemStackSummary) -> Result<Option<i32>> {
    let Some(patch) = static_item_component(item, VANILLA_DATA_COMPONENT_WEAPON_TYPE_ID)? else {
        return Ok(None);
    };
    patch
        .weapon_item_damage_per_attack
        .map(Some)
        .ok_or(ServerWorldError::MissingCatalogFact(
            "weapon item damage per attack",
        ))
}

/// `itemStack.getOrDefault(DataComponents.MINIMUM_ATTACK_CHARGE, 0.0F)`
/// (Player.java:1797-1801).
pub fn minimum_attack_charge(item: &ItemStackSummary) -> Result<f32> {
    let patch = &item.component_patch;
    if patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID)
    {
        return Ok(0.0);
    }
    if patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID)
    {
        let charge = patch.minimum_attack_charge_bits.map(f32::from_bits).ok_or(
            ServerWorldError::MissingCatalogFact("minimum attack charge patch value"),
        )?;
        if !charge.is_finite() || charge.total_cmp(&0.0).is_lt() || charge.total_cmp(&1.0).is_gt() {
            return Err(ServerWorldError::MissingCatalogFact(
                "valid minimum attack charge patch value",
            ));
        }
        return Ok(charge);
    }
    let Some(value) =
        prototype_component_object(item, VANILLA_DATA_COMPONENT_MINIMUM_ATTACK_CHARGE_TYPE_ID)?
    else {
        return Ok(0.0);
    };
    value
        .as_f64()
        .map(|charge| charge as f32)
        .ok_or(ServerWorldError::MissingCatalogFact(
            "minimum attack charge value",
        ))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MainHandAttackAttributes {
    pub damage: f64,
    pub speed: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainHandAttackDamageType {
    PlayerAttack,
    RegistryId(i32),
    ResourceId(&'static str),
}

/// Main-hand item modifiers projected onto the player's current attribute bases.
/// `LivingEntity.collectEquipmentChanges` applies groups that test MAINHAND, then
/// `AttributeInstance.calculateValue` evaluates operations in three phases
/// (LivingEntity.java:2883-2913; AttributeInstance.java:147-164).
pub fn main_hand_attack_attributes(
    item: &ItemStackSummary,
    base_damage: f64,
    base_speed: f64,
) -> Result<MainHandAttackAttributes> {
    if !base_damage.is_finite() || !base_speed.is_finite() {
        return Err(ServerWorldError::MissingCatalogFact(
            "finite player attack attribute base",
        ));
    }
    let modifiers = bbb_server_types::item_stack::static_item_component(
        item,
        VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    )?
    .map_or(&[][..], |component| {
        component.attribute_modifiers.as_slice()
    });
    if modifiers
        .iter()
        .any(|modifier| !f64::from_bits(modifier.amount_bits).is_finite())
    {
        return Err(ServerWorldError::MissingCatalogFact(
            "finite attribute modifier amount",
        ));
    }
    Ok(MainHandAttackAttributes {
        damage: compute_main_hand_attribute(
            base_damage,
            VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID,
            modifiers,
        )?,
        speed: compute_main_hand_attribute(
            base_speed,
            VANILLA_ATTRIBUTE_ATTACK_SPEED_ID,
            modifiers,
        )?,
    })
}

/// `ItemStack.getDamageSource` resolves the component before the item's own
/// source and the default player-attack source (ItemStack.java:1126-1130).
pub fn main_hand_attack_damage_type(item: &ItemStackSummary) -> Result<MainHandAttackDamageType> {
    let patch = &item.component_patch;
    if patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID)
    {
        return Ok(MainHandAttackDamageType::PlayerAttack);
    }
    if patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID)
    {
        let holder_id = patch
            .damage_type_id
            .ok_or(ServerWorldError::MissingCatalogFact(
                "attack damage type patch holder",
            ))?;
        if holder_id < 0 {
            return Err(ServerWorldError::MissingCatalogFact(
                "valid attack damage type patch holder",
            ));
        }
        return Ok(MainHandAttackDamageType::RegistryId(holder_id));
    }
    let Some(value) = prototype_component_object(item, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_TYPE_ID)?
    else {
        return Ok(MainHandAttackDamageType::PlayerAttack);
    };
    let resource_id = value
        .as_str()
        .filter(|resource_id| valid_resource_id(resource_id))
        .ok_or(ServerWorldError::MissingCatalogFact(
            "attack damage type prototype resource",
        ))?;
    Ok(MainHandAttackDamageType::ResourceId(resource_id))
}

fn compute_main_hand_attribute(
    base_value: f64,
    attribute_id: i32,
    modifiers: &[AttributeModifierSummary],
) -> Result<f64> {
    let mut effective = BTreeMap::new();
    for modifier in modifiers {
        if modifier.attribute_id == attribute_id
            && matches!(
                modifier.slot_id,
                VANILLA_EQUIPMENT_SLOT_GROUP_ANY_ID
                    | VANILLA_EQUIPMENT_SLOT_GROUP_MAINHAND_ID
                    | VANILLA_EQUIPMENT_SLOT_GROUP_HAND_ID
            )
        {
            effective.insert(
                modifier.modifier_id.as_str(),
                (modifier.operation_id, f64::from_bits(modifier.amount_bits)),
            );
        }
    }

    let mut base = base_value;
    for (operation_id, amount) in effective.values() {
        if *operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_VALUE_ID {
            base += amount;
        }
    }
    let mut result = base;
    for (operation_id, amount) in effective.values() {
        if *operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_BASE_ID {
            result += base * amount;
        }
    }
    for (operation_id, amount) in effective.values() {
        if *operation_id == VANILLA_ATTRIBUTE_MODIFIER_OPERATION_ADD_MULTIPLIED_TOTAL_ID {
            result *= 1.0 + amount;
        }
    }

    let attribute = vanilla_attribute_type(attribute_id).ok_or(
        ServerWorldError::MissingCatalogFact("player attack attribute type"),
    )?;
    let min = f64::from_bits(attribute.min_value_bits);
    let max = f64::from_bits(attribute.max_value_bits);
    Ok(if result.is_nan() {
        min
    } else {
        result.clamp(min, max)
    })
}

fn valid_resource_id(value: &str) -> bool {
    value
        .split_once(':')
        .is_some_and(|(namespace, path)| !namespace.is_empty() && !path.is_empty())
}

/// The prototype's value for a component whose fields this world reads, or
/// `None` when the stack has no readable one.
///
/// A patch that adds the component records only that it is there, so its value
/// is unknown; every caller treats that the same way it treats absence.
fn prototype_component_object(
    item: &ItemStackSummary,
    type_id: i32,
) -> Result<Option<&'static serde_json::Value>> {
    let Some(item_id) = item.item_id else {
        return Ok(None);
    };
    if item.component_patch.removed_type_ids.contains(&type_id)
        || item.component_patch.added_type_ids.contains(&type_id)
    {
        return Ok(None);
    }
    Ok(prototype_components(item_id)?.and_then(|components| components.get(&type_id)))
}

/// `JukeboxSong.fromStack` (`JukeboxSong.java:54-57`) against the actual publication.
pub fn jukebox_playable_song(
    item: &bbb_server_types::item_stack::RuntimeItemStack,
    resolver: &dyn bbb_registry_codec::PersistentItemStackHolderResolver,
) -> Result<Option<String>> {
    let Some(value) =
        item.effective_component(VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID, resolver)?
    else {
        return Ok(None);
    };
    let name = value
        .jukebox_song_id
        .and_then(|id| resolver.holder_resource_id("minecraft:jukebox_song", id))
        .ok_or(ServerWorldError::MissingCatalogFact(
            "jukebox playable song",
        ))?;
    Ok(Some(name))
}

/// `ItemInstance.getMaxStackSize` (ItemInstance.java:14-16).
pub fn max_stack_size(item: &ItemStackSummary) -> Result<i32> {
    bbb_server_types::item_stack::rules::max_stack_size(item)
}

/// `ItemStack.isDamageableItem` (ItemStack.java:417-419).
pub fn is_damageable_item(item: &ItemStackSummary) -> Result<bool> {
    Ok(
        has_component(item, VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID)?
            && !has_component(item, VANILLA_DATA_COMPONENT_UNBREAKABLE_TYPE_ID)?
            && has_component(item, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID)?,
    )
}

/// `ItemStack.isDamaged` (ItemStack.java:421-423).
pub fn is_damaged(item: &ItemStackSummary) -> Result<bool> {
    if !is_damageable_item(item)? {
        return Ok(false);
    }
    // `getDamageValue` clamps into the item's own damage range
    // (ItemStack.java:425-427), so a patch claiming more than the maximum
    // still only reports the maximum.
    let maximum = component_i32(
        item,
        VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
        item.component_patch.max_damage,
        0,
    )?;
    let damage = component_i32(
        item,
        VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
        item.component_patch.damage,
        0,
    )?;
    Ok(damage.clamp(0, maximum.max(0)) > 0)
}

/// `ItemStack.isStackable` (ItemStack.java:413-415).
pub fn is_stackable(item: &ItemStackSummary) -> Result<bool> {
    Ok(max_stack_size(item)? > 1 && !is_damaged(item)?)
}

/// `ItemStack.isSameItemSameComponents` (ItemStack.java:639-645). Two stacks of
/// the same item with the same patch resolve to the same components, so the
/// patch comparison is the component comparison.
pub fn is_same_item_same_components(a: &ItemStackSummary, b: &ItemStackSummary) -> bool {
    bbb_server_types::item_stack::rules::is_same_item_same_components(a, b)
}
