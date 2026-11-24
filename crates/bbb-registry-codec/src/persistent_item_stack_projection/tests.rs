use std::collections::BTreeMap;

use super::*;
mod adventure_mode_predicate_tests;
mod builtin_enum_component_tests;
mod equippable_component_tests;
mod holder_component_tests;
mod merchant_component_tests;
mod nested_item_stack_tests;
mod player_component_tests;
mod simple_record_component_tests;
mod source_template_tests;
mod tag_and_data_component_tests;
mod variant_component_tests;
mod weapon_and_block_attacks_tests;

use super::holder::{ENCHANTMENT_REGISTRY, MOB_EFFECT_REGISTRY};
use bbb_facts::block_types::{
    VANILLA_BLOCK_DIAMOND_ORE_ID, VANILLA_BLOCK_IRON_ORE_ID, VANILLA_BLOCK_STONE_ID,
};
use bbb_facts::data_component_types::{
    VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
    VANILLA_DATA_COMPONENT_ATTRIBUTE_MODIFIERS_TYPE_ID,
    VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
    VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
    VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
    VANILLA_DATA_COMPONENT_DAMAGE_RESISTANT_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
    VANILLA_DATA_COMPONENT_ENCHANTABLE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
    VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID, VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID,
    VANILLA_DATA_COMPONENT_PROFILE_TYPE_ID, VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
    VANILLA_DATA_COMPONENT_REPAIRABLE_TYPE_ID, VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
    VANILLA_DATA_COMPONENT_TYPES, VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID,
    VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID,
};
use bbb_facts::item_component_value_types::{FireworkExplosionShapeSummary, ItemRaritySummary};
use bbb_facts::item_types::{
    VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID, VANILLA_ITEM_BLADE_POTTERY_SHERD_ID,
    VANILLA_ITEM_BRICK_ID, VANILLA_ITEM_CHEST_ID, VANILLA_ITEM_COD_BUCKET_ID,
    VANILLA_ITEM_DIAMOND_PICKAXE_ID, VANILLA_ITEM_DIAMOND_SWORD_ID, VANILLA_ITEM_KNOWLEDGE_BOOK_ID,
    VANILLA_ITEM_STONE_ID, VANILLA_ITEM_WRITABLE_BOOK_ID, VANILLA_ITEM_WRITTEN_BOOK_ID,
};
use bbb_facts::mob_effect_types::{
    VANILLA_MOB_EFFECT_JUMP_BOOST_ID, VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
};
use bbb_facts::persistent_registry_facts_26_1::{
    vanilla_persistent_registry_membership_26_1, vanilla_persistent_registry_tag_members_26_1,
};
use bbb_facts::{
    PersistentRegistryMembership, PersistentRegistryResolver, PersistentRegistryTagMembers,
};
use bbb_protocol::nbt_summary::codec_int_value;
use bbb_protocol::packets::{
    DataComponentPatchSummary, FireworkExplosionSummary, ItemEnchantmentSummary,
    ItemMapDecorationSummary, ItemStackSummary, SuspiciousStewEffectSummary, ToolRuleSummary,
    ToolSummary, WrittenBookContentSummary,
};
use bbb_protocol::{component_literal_nbt, NbtSummaryEntry, NbtSummaryValue};

struct HolderResolver {
    ids: BTreeMap<(&'static str, &'static str), i32>,
}

impl PersistentRegistryResolver for HolderResolver {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if self.ids.contains_key(&(registry_id, resource_id)) {
            PersistentRegistryMembership::Present
        } else {
            PersistentRegistryMembership::Absent
        }
    }

    fn tag_members(&self, _registry_id: &str, _tag_id: &str) -> PersistentRegistryTagMembers {
        PersistentRegistryTagMembers::Unavailable
    }
}

impl PersistentItemStackHolderResolver for HolderResolver {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.ids.get(&(registry_id, resource_id)).copied()
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.ids
            .iter()
            .find(|((registry, _), id)| *registry == registry_id && **id == holder_id)
            .map(|((_, resource_id), _)| (*resource_id).to_string())
    }
}

/// The built-in 26.1 registry snapshot, which is what a persistent value
/// projected out of a vanilla live stack has to validate against.
struct BuiltinResolver;

impl PersistentRegistryResolver for BuiltinResolver {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        vanilla_persistent_registry_membership_26_1(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        vanilla_persistent_registry_tag_members_26_1(registry_id, tag_id)
    }
}

impl PersistentItemStackHolderResolver for BuiltinResolver {
    fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
        None
    }
}

/// A publication that claims every resource, which is the only way to reach the
/// projection's own built-in-catalog check for a holder the session accepted.
struct PermissiveResolver;

impl PersistentRegistryResolver for PermissiveResolver {
    fn resource_membership(
        &self,
        _registry_id: &str,
        _resource_id: &str,
    ) -> PersistentRegistryMembership {
        PersistentRegistryMembership::Present
    }

    fn tag_members(&self, _registry_id: &str, _tag_id: &str) -> PersistentRegistryTagMembers {
        PersistentRegistryTagMembers::Present(Vec::new())
    }
}

impl PersistentItemStackHolderResolver for PermissiveResolver {
    fn holder_id(&self, _registry_id: &str, _resource_id: &str) -> Option<i32> {
        None
    }
}

/// A publication that names the pairs it was built with and answers the
/// built-in 26.1 facts otherwise, which is what a session resolver is.
pub(super) struct PublishingResolver {
    ids: BTreeMap<(&'static str, &'static str), i32>,
}

impl PersistentRegistryResolver for PublishingResolver {
    fn resource_membership(
        &self,
        registry_id: &str,
        resource_id: &str,
    ) -> PersistentRegistryMembership {
        if self.ids.contains_key(&(registry_id, resource_id)) {
            return PersistentRegistryMembership::Present;
        }
        vanilla_persistent_registry_membership_26_1(registry_id, resource_id)
    }

    fn tag_members(&self, registry_id: &str, tag_id: &str) -> PersistentRegistryTagMembers {
        vanilla_persistent_registry_tag_members_26_1(registry_id, tag_id)
    }
}

impl PersistentItemStackHolderResolver for PublishingResolver {
    fn holder_id(&self, registry_id: &str, resource_id: &str) -> Option<i32> {
        self.ids.get(&(registry_id, resource_id)).copied()
    }

    fn holder_resource_id(&self, registry_id: &str, holder_id: i32) -> Option<String> {
        self.ids
            .iter()
            .find(|((registry, _), id)| *registry == registry_id && **id == holder_id)
            .map(|((_, resource_id), _)| (*resource_id).to_string())
    }
}

pub(super) fn publishing(pairs: &[(&'static str, &'static str, i32)]) -> PublishingResolver {
    PublishingResolver {
        ids: pairs
            .iter()
            .map(|&(registry, resource, id)| ((registry, resource), id))
            .collect(),
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn stack(item_id: i32, count: i32, component_patch: DataComponentPatchSummary) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: component_patch.into(),
    }
}

fn added_patch(type_ids: &[i32]) -> DataComponentPatchSummary {
    DataComponentPatchSummary {
        added_type_ids: type_ids.to_vec(),
        ..DataComponentPatchSummary::default()
    }
}

fn custom_data_tag() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![entry("answer", NbtSummaryValue::Int(42))])
}

fn saved_stack_with_components(item: &str, components: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("id", NbtSummaryValue::String(item.to_string().into())),
        entry("count", NbtSummaryValue::Int(1)),
        entry("components", NbtSummaryValue::Compound(components)),
    ])
}

/// The `components` compound of a projected stack, or `None` when the field
/// was omitted.
fn projected_components(nbt: &NbtSummaryValue) -> Option<&NbtSummaryValue> {
    let NbtSummaryValue::Compound(entries) = nbt else {
        panic!("projected stack is not a compound");
    };
    entries
        .iter()
        .find(|entry| entry.name == "components")
        .map(|entry| &entry.value)
}

#[test]
fn empty_component_patch_round_trips_through_the_persistent_decoder() {
    let summary = stack(
        VANILLA_ITEM_STONE_ID,
        7,
        DataComponentPatchSummary::default(),
    );
    let nbt = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(projected_components(&nbt), None);

    assert_eq!(
        crate::read_item_stack(&nbt, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
}

/// `id` is the item's namespaced resource location as a string and `count` is
/// an int that is written even at the codec's decode default of one.
///
/// `Item.CODEC` is `BuiltInRegistries.ITEM.holderByNameCodec()`
/// and `Identifier.toString` always emits
/// `namespace:path`, so `minecraft:` is never
/// elided. `count` is `ExtraCodecs.intRange(1, 99)`, i.e. `Codec.INT`
/// its `orElse(1)` is a
/// decode-side default, so the encoded form still carries the field.
#[test]
fn persistent_stack_writes_a_string_id_and_an_int_count() {
    let nbt = item_stack_summary_to_persistent_nbt(&stack(
        VANILLA_ITEM_STONE_ID,
        1,
        DataComponentPatchSummary::default(),
    ))
    .unwrap();
    assert_eq!(
        nbt,
        NbtSummaryValue::Compound(vec![
            entry(
                "id",
                NbtSummaryValue::String("minecraft:stone".to_string().into())
            ),
            entry("count", NbtSummaryValue::Int(1)),
        ])
    );
}

/// The projection's coverage is enumerated from the generated component
/// catalog rather than declared by hand: every entry falls into exactly one of
/// transient / retained / typed / lossy, and each lossy one really does fail
/// closed.
///
/// The three transient entries are the components vanilla itself refuses to
/// persist -- `DataComponentPatch.CODEC` drops every `type.isTransient()`
/// entry while encoding and `PatchKey.CODEC`
/// rejects one while parsing -- and they
/// are the only ones left outside the two projecting sets: the lossy set is
/// empty.
#[test]
fn persistent_item_stack_projection_coverage_is_enumerated_from_the_catalog() {
    let mut transient = Vec::new();
    let mut retained = Vec::new();
    let mut typed = Vec::new();
    let mut lossy = Vec::new();
    for component in VANILLA_DATA_COMPONENT_TYPES {
        if !component.persistent {
            transient.push(component.resource_id);
        } else if component.type_id == VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID
            || PERSISTENT_CODEC_BACKED_COMPONENT_TYPE_IDS.contains(&component.type_id)
        {
            retained.push(component.resource_id);
        } else if TYPED_PERSISTENT_ENCODED_COMPONENT_TYPE_IDS.contains(&component.type_id) {
            typed.push(component.resource_id);
        } else {
            lossy.push(component);
        }
    }

    assert_eq!(VANILLA_DATA_COMPONENT_TYPES.len(), 110);
    assert_eq!(
        transient,
        [
            "minecraft:creative_slot_lock",
            "minecraft:additional_trade_cost",
            "minecraft:map_post_processing",
        ]
    );
    assert_eq!(
        retained,
        [
            "minecraft:custom_data",
            "minecraft:map_decorations",
            "minecraft:bucket_entity_data",
            "minecraft:recipes",
            "minecraft:container_loot",
        ]
    );
    assert_eq!(
        typed,
        [
            "minecraft:max_stack_size",
            "minecraft:max_damage",
            "minecraft:damage",
            "minecraft:unbreakable",
            "minecraft:use_effects",
            "minecraft:custom_name",
            "minecraft:minimum_attack_charge",
            "minecraft:damage_type",
            "minecraft:item_name",
            "minecraft:item_model",
            "minecraft:lore",
            "minecraft:rarity",
            "minecraft:enchantments",
            "minecraft:can_place_on",
            "minecraft:can_break",
            "minecraft:attribute_modifiers",
            "minecraft:custom_model_data",
            "minecraft:tooltip_display",
            "minecraft:repair_cost",
            "minecraft:enchantment_glint_override",
            "minecraft:intangible_projectile",
            "minecraft:food",
            "minecraft:consumable",
            "minecraft:use_remainder",
            "minecraft:use_cooldown",
            "minecraft:damage_resistant",
            "minecraft:tool",
            "minecraft:weapon",
            "minecraft:attack_range",
            "minecraft:enchantable",
            "minecraft:equippable",
            "minecraft:repairable",
            "minecraft:glider",
            "minecraft:tooltip_style",
            "minecraft:death_protection",
            "minecraft:blocks_attacks",
            "minecraft:piercing_weapon",
            "minecraft:kinetic_weapon",
            "minecraft:swing_animation",
            "minecraft:stored_enchantments",
            "minecraft:dye",
            "minecraft:dyed_color",
            "minecraft:map_color",
            "minecraft:map_id",
            "minecraft:charged_projectiles",
            "minecraft:bundle_contents",
            "minecraft:potion_contents",
            "minecraft:potion_duration_scale",
            "minecraft:suspicious_stew_effects",
            "minecraft:writable_book_content",
            "minecraft:written_book_content",
            "minecraft:trim",
            "minecraft:debug_stick_state",
            "minecraft:entity_data",
            "minecraft:block_entity_data",
            "minecraft:instrument",
            "minecraft:provides_trim_material",
            "minecraft:ominous_bottle_amplifier",
            "minecraft:jukebox_playable",
            "minecraft:provides_banner_patterns",
            "minecraft:lodestone_tracker",
            "minecraft:firework_explosion",
            "minecraft:fireworks",
            "minecraft:profile",
            "minecraft:note_block_sound",
            "minecraft:banner_patterns",
            "minecraft:base_color",
            "minecraft:pot_decorations",
            "minecraft:container",
            "minecraft:block_state",
            "minecraft:bees",
            "minecraft:lock",
            "minecraft:break_sound",
            "minecraft:villager/variant",
            "minecraft:wolf/variant",
            "minecraft:wolf/sound_variant",
            "minecraft:wolf/collar",
            "minecraft:fox/variant",
            "minecraft:salmon/size",
            "minecraft:parrot/variant",
            "minecraft:tropical_fish/pattern",
            "minecraft:tropical_fish/base_color",
            "minecraft:tropical_fish/pattern_color",
            "minecraft:mooshroom/variant",
            "minecraft:rabbit/variant",
            "minecraft:pig/variant",
            "minecraft:pig/sound_variant",
            "minecraft:cow/variant",
            "minecraft:cow/sound_variant",
            "minecraft:chicken/variant",
            "minecraft:chicken/sound_variant",
            "minecraft:zombie_nautilus/variant",
            "minecraft:frog/variant",
            "minecraft:horse/variant",
            "minecraft:painting/variant",
            "minecraft:llama/variant",
            "minecraft:axolotl/variant",
            "minecraft:cat/variant",
            "minecraft:cat/sound_variant",
            "minecraft:cat/collar",
            "minecraft:sheep/color",
            "minecraft:shulker/color",
        ]
    );
    assert!(lossy.is_empty(), "{:?}", lossy);
    assert_eq!(
        transient.len() + retained.len() + typed.len() + lossy.len(),
        VANILLA_DATA_COMPONENT_TYPES.len()
    );

    for component in lossy {
        let projected = item_stack_summary_to_persistent_nbt(&stack(
            VANILLA_ITEM_STONE_ID,
            1,
            added_patch(&[component.type_id]),
        ));
        assert_eq!(
            projected,
            Err(ItemStackPersistentProjectionError::UnsupportedComponent {
                type_id: component.type_id,
                resource_id: component.resource_id,
            }),
            "{} should have no persistent projection",
            component.resource_id
        );
    }
}

/// An empty stack has no `ItemStack.CODEC` encoding at all, and
/// `ItemStack.OPTIONAL_CODEC` spells it as an empty compound.
///
/// `Item.CODEC` rejects `minecraft:air` and the count
/// range starts at one, so `ItemStack.EMPTY` cannot go
/// through `ItemStack.CODEC` -- which is why
/// `ItemEntity.addAdditionalSaveData` guards the whole `Item` field behind
/// `isEmpty`. `OPTIONAL_CODEC` instead maps it to
/// `ExtraCodecs.optionalEmptyMap`'s `ops.emptyMap()` (`ItemStack`,
/// `ExtraCodecs`), and the repository's decoder reads that empty
/// compound back as the empty stack.
#[test]
fn empty_stacks_have_no_codec_encoding_but_an_empty_optional_map() {
    let air = stack(
        bbb_facts::item_types::VANILLA_ITEM_AIR_ID,
        1,
        DataComponentPatchSummary::default(),
    );
    let zero_count = stack(
        VANILLA_ITEM_STONE_ID,
        0,
        DataComponentPatchSummary::default(),
    );
    for summary in [ItemStackSummary::empty(), air, zero_count] {
        assert!(item_stack_summary_is_empty(&summary));
        assert_eq!(
            item_stack_summary_to_persistent_nbt(&summary),
            Err(ItemStackPersistentProjectionError::EmptyStack)
        );
    }
}

/// A removed component is keyed by `"!" + id` and carries an empty compound.
///
/// `PatchKey.CODEC` writes the `!` prefix
/// and a removed key's value codec is `Codec.EMPTY.codec()`
/// which encodes to an empty map.
#[test]
fn removed_components_are_written_with_a_bang_prefix_and_an_empty_compound() {
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID]);
    patch.custom_data = Some(custom_data_tag());
    patch.removed_type_ids = vec![VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID];

    let nbt = item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_DIAMOND_SWORD_ID, 1, patch))
        .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![
            entry("minecraft:custom_data", custom_data_tag()),
            entry(
                "!minecraft:max_stack_size",
                NbtSummaryValue::Compound(Vec::new())
            ),
        ]))
    );
}

/// Transient components are dropped from `components`, but their presence
/// still keeps the field from being omitted.
///
/// The encode-side `xmap` of `DataComponentPatch.CODEC` skips every
/// `type.isTransient()` entry, so a
/// transient-only patch produces an empty dispatched map. The decision to omit
/// `components` is taken earlier and on the *unfiltered* patch:
/// `optionalFieldOf` compares it with
/// `DataComponentPatch.EMPTY` using `equals`,
/// which compares the raw maps. A patch
/// holding only a transient entry is therefore not the default, and vanilla
/// writes `components:{}`.
#[test]
fn transient_components_are_dropped_but_still_force_a_components_field() {
    for type_id in [
        VANILLA_DATA_COMPONENT_CREATIVE_SLOT_LOCK_TYPE_ID,
        VANILLA_DATA_COMPONENT_ADDITIONAL_TRADE_COST_TYPE_ID,
        VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID,
    ] {
        let nbt = item_stack_summary_to_persistent_nbt(&stack(
            VANILLA_ITEM_STONE_ID,
            1,
            added_patch(&[type_id]),
        ))
        .unwrap();
        assert_eq!(
            projected_components(&nbt),
            Some(&NbtSummaryValue::Compound(Vec::new()))
        );
        assert_eq!(
            crate::read_item_stack(&nbt, &crate::UnavailableHolderResolver)
                .unwrap()
                .component_patch,
            DataComponentPatchSummary::default()
        );
    }

    let removal_only = DataComponentPatchSummary {
        removed_type_ids: vec![VANILLA_DATA_COMPONENT_MAP_POST_PROCESSING_TYPE_ID],
        ..Default::default()
    };
    let nbt = item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_STONE_ID, 1, removal_only))
        .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(Vec::new()))
    );
}

/// Each retained component projects to the value its Vanilla persistent codec
/// writes, and the result passes the repository's deep persistent validator.
///
/// `minecraft:custom_data` is a bare `CompoundTag` on both sides
/// `minecraft:container_loot` is `persistent(SeededContainerLoot.CODEC)` with
/// no network codec of its own;
/// `minecraft:recipes` is `persistent(Recipe.KEY_CODEC.listOf())`
/// and `minecraft:bucket_entity_data` shares
/// `CustomData` with both codecs. In every one
/// of those cases the wire value bbb retained is already the save value.
#[test]
fn retained_codec_components_project_to_their_vanilla_save_shape() {
    let mut custom = added_patch(&[VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID]);
    custom.custom_data = Some(custom_data_tag());
    let nbt =
        item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_STONE_ID, 3, custom)).unwrap();
    assert_eq!(
        nbt,
        NbtSummaryValue::Compound(vec![
            entry(
                "id",
                NbtSummaryValue::String("minecraft:stone".to_string().into())
            ),
            entry("count", NbtSummaryValue::Int(3)),
            entry(
                "components",
                NbtSummaryValue::Compound(vec![entry("minecraft:custom_data", custom_data_tag())])
            ),
        ])
    );

    let loot = NbtSummaryValue::Compound(vec![
        entry(
            "loot_table",
            NbtSummaryValue::String("minecraft:chests/simple_dungeon".to_string().into()),
        ),
        entry("seed", NbtSummaryValue::Long(1234)),
    ]);
    let mut container_loot = added_patch(&[VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID]);
    container_loot.container_loot = true;
    container_loot.container_loot_table = Some("minecraft:chests/simple_dungeon".to_string());
    container_loot.container_loot_seed = Some(1234);
    container_loot.codec_backed_nbt =
        BTreeMap::from([(VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID, loot.clone())]);
    let nbt =
        item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_CHEST_ID, 1, container_loot))
            .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:container_loot",
            loot
        )]))
    );

    let recipes = NbtSummaryValue::List(vec![
        NbtSummaryValue::String("minecraft:stick".to_string().into()),
        NbtSummaryValue::String("minecraft:torch".to_string().into()),
    ]);
    let mut recipe_patch = added_patch(&[VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID]);
    recipe_patch.recipe_keys = Some(vec![
        "minecraft:stick".to_string(),
        "minecraft:torch".to_string(),
    ]);
    recipe_patch.codec_backed_nbt =
        BTreeMap::from([(VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, recipes.clone())]);
    let nbt = item_stack_summary_to_persistent_nbt(&stack(
        VANILLA_ITEM_KNOWLEDGE_BOOK_ID,
        1,
        recipe_patch,
    ))
    .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:recipes",
            recipes
        )]))
    );

    let bucket_tag = NbtSummaryValue::Compound(vec![entry(
        "Health",
        NbtSummaryValue::Float(3.0f32.to_bits()),
    )]);
    let mut bucket = added_patch(&[VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID]);
    bucket.codec_backed_nbt = BTreeMap::from([(
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
        bucket_tag.clone(),
    )]);
    let nbt = item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_COD_BUCKET_ID, 1, bucket))
        .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:bucket_entity_data",
            bucket_tag
        )]))
    );
}

/// `ItemStack.OPTIONAL_STREAM_CODEC`'s decode maps a stack that is not there to
/// `ItemStack.EMPTY`, which is what the absent-item
/// save form spells.
#[test]
fn an_absent_saved_stack_projects_to_the_empty_summary() {
    assert_eq!(
        read_item_stack(
            &NbtSummaryValue::Compound(Vec::new()),
            &UnavailableHolderResolver,
        ),
        Ok(ItemStackSummary::empty())
    );
}

/// The five component codecs retained by the wire decoder project from their
/// saved NBT into the same retained and typed summary fields.
#[test]
fn retained_saved_components_project_to_the_wire_decoder_shape() {
    let map_decorations = NbtSummaryValue::Compound(vec![entry(
        "spawn",
        NbtSummaryValue::Compound(vec![
            entry(
                "type",
                NbtSummaryValue::String("minecraft:player".to_string().into()),
            ),
            entry("x", NbtSummaryValue::Double(1.25f64.to_bits())),
            entry("z", NbtSummaryValue::Double((-3.5f64).to_bits())),
            entry("rotation", NbtSummaryValue::Float(90.0f32.to_bits())),
        ]),
    )]);
    let bucket_data = NbtSummaryValue::Compound(vec![entry(
        "Health",
        NbtSummaryValue::Float(3.0f32.to_bits()),
    )]);
    let recipes = NbtSummaryValue::List(vec![
        NbtSummaryValue::String("minecraft:stick".to_string().into()),
        NbtSummaryValue::String("torch".to_string().into()),
    ]);
    let canonical_recipes = NbtSummaryValue::List(vec![
        NbtSummaryValue::String("minecraft:stick".to_string().into()),
        NbtSummaryValue::String("minecraft:torch".to_string().into()),
    ]);
    let container_loot = NbtSummaryValue::Compound(vec![
        entry(
            "loot_table",
            NbtSummaryValue::String("minecraft:chests/simple_dungeon".to_string().into()),
        ),
        entry("seed", NbtSummaryValue::Long(1234)),
    ]);
    let saved = saved_stack_with_components(
        "minecraft:chest",
        vec![
            entry("minecraft:container_loot", container_loot.clone()),
            entry("minecraft:custom_data", custom_data_tag()),
            entry("minecraft:recipes", recipes),
            entry("minecraft:map_decorations", map_decorations.clone()),
            entry("minecraft:bucket_entity_data", bucket_data.clone()),
        ],
    );
    let summary = read_item_stack(&saved, &UnavailableHolderResolver).unwrap();
    assert_eq!(summary.item_id, Some(VANILLA_ITEM_CHEST_ID));
    assert_eq!(summary.count, 1);
    assert_eq!(summary.component_patch.added_type_ids.len(), 5);
    assert_eq!(
        summary.component_patch.added_type_ids,
        vec![
            VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
            VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
            VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID,
            VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
        ]
    );
    assert!(summary.component_patch.removed_type_ids.is_empty());
    assert_eq!(summary.component_patch.custom_data, Some(custom_data_tag()));
    assert_eq!(
        summary.component_patch.codec_backed_nbt,
        BTreeMap::from([
            (
                VANILLA_DATA_COMPONENT_MAP_DECORATIONS_TYPE_ID,
                map_decorations,
            ),
            (
                VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID,
                bucket_data,
            ),
            (VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, canonical_recipes),
            (
                VANILLA_DATA_COMPONENT_CONTAINER_LOOT_TYPE_ID,
                container_loot,
            ),
        ])
    );
    assert_eq!(
        summary.component_patch.map_decorations,
        Some(vec![ItemMapDecorationSummary {
            id: "spawn".into(),
            decoration_type_id: 0,
            decoration_type_resource_id: "minecraft:player".to_string(),
            x_bits: 1.25f64.to_bits(),
            z_bits: (-3.5f64).to_bits(),
            rotation_bits: 90.0f32.to_bits(),
        }])
    );
    assert_eq!(
        summary.component_patch.recipe_keys,
        Some(vec![
            "minecraft:stick".to_string(),
            "minecraft:torch".to_string(),
        ])
    );
    assert!(summary.component_patch.container_loot);
    assert_eq!(
        summary.component_patch.container_loot_table.as_deref(),
        Some("minecraft:chests/simple_dungeon")
    );
    assert_eq!(summary.component_patch.container_loot_seed, Some(1234));
}

/// The read contract's canonical order: the same stack written with its map
/// entries and components in another order reads to an equal summary.
#[test]
fn map_order_does_not_change_the_read_summary() {
    let resolver = HolderResolver {
        ids: BTreeMap::from([
            ((ENCHANTMENT_REGISTRY, "minecraft:sharpness"), 13),
            ((ENCHANTMENT_REGISTRY, "minecraft:unbreaking"), 7),
        ]),
    };
    let read = |enchantments: Vec<NbtSummaryEntry>, damage_first: bool| {
        let mut components = vec![
            entry(
                "minecraft:enchantments",
                NbtSummaryValue::Compound(enchantments),
            ),
            entry("minecraft:damage", NbtSummaryValue::Int(3)),
        ];
        if damage_first {
            components.reverse();
        }
        crate::read_item_stack(
            &saved_stack_with_components("minecraft:diamond_sword", components),
            &resolver,
        )
        .unwrap()
    };
    let sharpness = || entry("minecraft:sharpness", NbtSummaryValue::Int(5));
    let unbreaking = || entry("minecraft:unbreaking", NbtSummaryValue::Int(2));
    assert_eq!(
        read(vec![sharpness(), unbreaking()], false),
        read(vec![unbreaking(), sharpness()], true)
    );
}

/// `minecraft:damage` is `persistent(ExtraCodecs.NON_NEGATIVE_INT)`
/// an int the summary already holds.
///
/// An ordinary survival break writes one onto the held tool, so a live
/// inventory carrying it has to reach the save file.
#[test]
fn damaged_tools_project_the_persistent_damage_int() {
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID]);
    patch.damage = Some(137);
    let nbt =
        item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_DIAMOND_PICKAXE_ID, 1, patch))
            .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:damage",
            NbtSummaryValue::Int(137)
        )]))
    );

    let saved = nbt.clone();
    assert_eq!(
        crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap(),
        stack(VANILLA_ITEM_DIAMOND_PICKAXE_ID, 1, {
            let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID]);
            patch.damage = Some(137);
            patch
        })
    );
}

/// `ItemEnchantments.CODEC` keys its map by each holder's identifier
/// so the projection
/// round-trips only through the publication that issued the summary's ids.
#[test]
fn enchantments_round_trip_through_the_publication_that_named_them() {
    let resolver = HolderResolver {
        ids: BTreeMap::from([
            ((ENCHANTMENT_REGISTRY, "minecraft:efficiency"), 11),
            ((ENCHANTMENT_REGISTRY, "minecraft:unbreaking"), 27),
        ]),
    };
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID]);
    patch.enchantments = vec![
        ItemEnchantmentSummary {
            holder_id: 11,
            level: 4,
        },
        ItemEnchantmentSummary {
            holder_id: 27,
            level: 3,
        },
    ];
    let summary = stack(VANILLA_ITEM_DIAMOND_PICKAXE_ID, 1, patch);

    let nbt = item_stack_summary_to_persistent_nbt_with_resolver(&summary, &resolver).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:enchantments",
            NbtSummaryValue::Compound(vec![
                entry("minecraft:efficiency", NbtSummaryValue::Int(4)),
                entry("minecraft:unbreaking", NbtSummaryValue::Int(3)),
            ])
        )]))
    );

    let saved = nbt.clone();
    assert_eq!(crate::read_item_stack(&saved, &resolver).unwrap(), summary);
}

/// `ItemEnchantments.EMPTY` is the empty map, which
/// needs no publication to write.
#[test]
fn empty_enchantments_project_the_empty_item_enchantments_map() {
    let nbt = item_stack_summary_to_persistent_nbt(&stack(
        VANILLA_ITEM_DIAMOND_PICKAXE_ID,
        1,
        added_patch(&[VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID]),
    ))
    .unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:enchantments",
            NbtSummaryValue::Compound(Vec::new())
        )]))
    );
}

fn tool_rule(
    block_registry_ids: Vec<i32>,
    block_tag: Option<&str>,
    speed: Option<f32>,
    correct_for_drops: Option<bool>,
) -> ToolRuleSummary {
    ToolRuleSummary {
        block_registry_ids,
        block_tag: block_tag.map(str::to_string),
        speed_bits: speed.map(f32::to_bits),
        correct_for_drops,
    }
}

/// `Tool.CODEC` writes `rules` alone when the other three fields sit at their
/// `optionalFieldOf` defaults, and each rule's block set is
/// a `"#tag"`, a bare identifier for one holder, or a list
#[test]
fn tool_rules_project_every_holder_set_shape_and_omit_codec_defaults() {
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_TOOL_TYPE_ID]);
    patch.tool = Some(ToolSummary {
        rules: vec![
            tool_rule(
                Vec::new(),
                Some("minecraft:mineable/pickaxe"),
                Some(8.0),
                Some(true),
            ),
            tool_rule(vec![VANILLA_BLOCK_STONE_ID], None, None, Some(false)),
            tool_rule(
                vec![VANILLA_BLOCK_IRON_ORE_ID, VANILLA_BLOCK_DIAMOND_ORE_ID],
                None,
                Some(1.0),
                None,
            ),
        ],
        default_mining_speed_bits: 1.0_f32.to_bits(),
        damage_per_block: 1,
        can_destroy_blocks_in_creative: true,
    });
    let summary = stack(VANILLA_ITEM_DIAMOND_PICKAXE_ID, 1, patch);
    let nbt = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:tool",
            NbtSummaryValue::Compound(vec![entry(
                "rules",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "blocks",
                            NbtSummaryValue::String(
                                "#minecraft:mineable/pickaxe".to_string().into()
                            )
                        ),
                        entry("speed", NbtSummaryValue::Float(8.0_f32.to_bits())),
                        entry("correct_for_drops", NbtSummaryValue::Byte(1)),
                    ]),
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "blocks",
                            NbtSummaryValue::String("minecraft:stone".to_string().into())
                        ),
                        entry("correct_for_drops", NbtSummaryValue::Byte(0)),
                    ]),
                    NbtSummaryValue::Compound(vec![
                        entry(
                            "blocks",
                            NbtSummaryValue::List(vec![
                                NbtSummaryValue::String("minecraft:iron_ore".to_string().into()),
                                NbtSummaryValue::String("minecraft:diamond_ore".to_string().into()),
                            ])
                        ),
                        entry("speed", NbtSummaryValue::Float(1.0_f32.to_bits())),
                    ]),
                ])
            )])
        )]))
    );

    let saved = nbt.clone();
    assert_eq!(
        crate::read_item_stack(&saved, &BuiltinResolver).unwrap(),
        summary
    );
}

/// A non-default `default_mining_speed` / `damage_per_block` /
/// `can_destroy_blocks_in_creative` is written, and `Codec.BOOL` reaches NBT as
/// a `ByteTag`.
#[test]
fn tool_scalars_away_from_their_defaults_are_written() {
    let zero_speed = NbtSummaryValue::Compound(vec![entry(
        "rules",
        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
            entry("blocks", NbtSummaryValue::List(Vec::new())),
            entry("speed", NbtSummaryValue::Float(0.0_f32.to_bits())),
        ])]),
    )]);
    assert!(read_item_component(
        VANILLA_DATA_COMPONENT_TOOL_TYPE_ID,
        &zero_speed,
        &BuiltinResolver
    )
    .is_err());
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_TOOL_TYPE_ID]);
    patch.tool = Some(ToolSummary {
        rules: Vec::new(),
        default_mining_speed_bits: 2.5_f32.to_bits(),
        damage_per_block: 3,
        can_destroy_blocks_in_creative: false,
    });
    let summary = stack(VANILLA_ITEM_DIAMOND_PICKAXE_ID, 1, patch);
    let nbt = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:tool",
            NbtSummaryValue::Compound(vec![
                entry("rules", NbtSummaryValue::List(Vec::new())),
                entry(
                    "default_mining_speed",
                    NbtSummaryValue::Float(2.5_f32.to_bits())
                ),
                entry("damage_per_block", NbtSummaryValue::Int(3)),
                entry("can_destroy_blocks_in_creative", NbtSummaryValue::Byte(0)),
            ])
        )]))
    );

    let saved = nbt.clone();
    assert_eq!(
        crate::read_item_stack(&saved, &BuiltinResolver).unwrap(),
        summary
    );
}

/// Removals need no value conversion, so every known persistent component can
/// be represented on the wire and the output remains in type-id order.
#[test]
fn persistent_component_removals_project_without_loss_in_deterministic_order() {
    let saved = saved_stack_with_components(
        "minecraft:diamond_sword",
        vec![
            entry("!minecraft:damage", NbtSummaryValue::Compound(Vec::new())),
            entry(
                "!minecraft:max_stack_size",
                NbtSummaryValue::Compound(Vec::new()),
            ),
        ],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(summary.component_patch.added_type_ids.len(), 0);
    assert!(summary.component_patch.added_type_ids.is_empty());
    assert_eq!(
        summary.component_patch,
        DataComponentPatchSummary {
            removed_type_ids: vec![
                VANILLA_DATA_COMPONENT_MAX_STACK_SIZE_TYPE_ID,
                VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            ],
            ..DataComponentPatchSummary::default()
        }
    );
}

#[test]
fn map_id_round_trips_through_the_typed_wire_projection() {
    let saved = saved_stack_with_components(
        "minecraft:filled_map",
        vec![entry("minecraft:map_id", NbtSummaryValue::Int(17))],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(summary.component_patch.map_id, Some(17));
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

/// `OminousBottleAmplifier.CODEC` is `ExtraCodecs.intRange(0, 4).xmap`
/// so the pillager captain's
/// `set_ominous_bottle_amplifier` drop saves as a bare int and reads back.
#[test]
fn the_ominous_bottle_amplifier_round_trips_as_a_bare_int() {
    for amplifier in [0, 3, 4] {
        let saved = saved_stack_with_components(
            "minecraft:ominous_bottle",
            vec![entry(
                "minecraft:ominous_bottle_amplifier",
                NbtSummaryValue::Int(amplifier),
            )],
        );
        let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
        assert_eq!(
            summary.component_patch.ominous_bottle_amplifier,
            Some(amplifier)
        );
        assert_eq!(
            item_stack_summary_to_persistent_nbt(&summary).unwrap(),
            saved
        );
    }
}

/// The scalar components whose persistent codec is one int, one float or one
/// unit value round-trip through exactly that save shape.
///
/// * `map_color` is `Codec.INT.xmap`;
/// * `minimum_attack_charge` is `floatRange(0.0F, 1.0F)`
/// * `potion_duration_scale` is `NON_NEGATIVE_FLOAT`
/// * `intangible_projectile` is `MapCodec.unitCodec`, whose
///   only save value is the empty compound.
#[test]
fn scalar_int_float_and_unit_components_round_trip() {
    let saved = saved_stack_with_components(
        "minecraft:filled_map",
        vec![
            entry(
                "minecraft:minimum_attack_charge",
                NbtSummaryValue::Float(0.5_f32.to_bits()),
            ),
            entry(
                "minecraft:intangible_projectile",
                NbtSummaryValue::Compound(Vec::new()),
            ),
            entry("minecraft:map_color", NbtSummaryValue::Int(4_603_950)),
            entry(
                "minecraft:potion_duration_scale",
                NbtSummaryValue::Float(0.125_f32.to_bits()),
            ),
        ],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        summary.component_patch.map_color,
        Some(4_603_950),
        "map colour"
    );
    assert_eq!(
        summary.component_patch.minimum_attack_charge_bits,
        Some(0.5_f32.to_bits())
    );
    assert_eq!(
        summary.component_patch.potion_duration_scale_bits,
        Some(0.125_f32.to_bits())
    );
    assert!(summary.component_patch.intangible_projectile);
    assert_eq!(summary.component_patch.added_type_ids.len(), 4);
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

/// The structured components whose persistent codec the summary's typed
/// fields already carry round-trip through that save shape:
///
/// * `custom_model_data` -- four optional lists;
/// * `tooltip_display` -- `hide_tooltip` and `hidden_components`
/// * `use_cooldown` -- a required `seconds` and an optional `cooldown_group`
/// * `weapon` -- `item_damage_per_attack` and `disable_blocking_for_seconds`
/// * `firework_explosion` and `fireworks` -- the explosion record and the list
///   of them;
/// * `block_state` -- a string map.
#[test]
fn structured_record_components_round_trip_together() {
    let saved = saved_stack_with_components(
        "minecraft:firework_rocket",
        vec![
            entry(
                "minecraft:custom_model_data",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "floats",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Float(0.5_f32.to_bits())]),
                    ),
                    entry(
                        "flags",
                        NbtSummaryValue::List(vec![
                            NbtSummaryValue::Byte(1),
                            NbtSummaryValue::Byte(0),
                        ]),
                    ),
                    entry(
                        "strings",
                        NbtSummaryValue::List(vec![NbtSummaryValue::String(
                            "bbb".to_string().into(),
                        )]),
                    ),
                    entry(
                        "colors",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Int(0x11_22_33)]),
                    ),
                ]),
            ),
            entry(
                "minecraft:tooltip_display",
                NbtSummaryValue::Compound(vec![
                    entry("hide_tooltip", NbtSummaryValue::Byte(1)),
                    entry(
                        "hidden_components",
                        NbtSummaryValue::List(vec![NbtSummaryValue::String(
                            "minecraft:enchantments".to_string().into(),
                        )]),
                    ),
                ]),
            ),
            entry(
                "minecraft:use_cooldown",
                NbtSummaryValue::Compound(vec![
                    entry("seconds", NbtSummaryValue::Float(1.5_f32.to_bits())),
                    entry(
                        "cooldown_group",
                        NbtSummaryValue::String("example:group".to_string().into()),
                    ),
                ]),
            ),
            entry(
                "minecraft:weapon",
                NbtSummaryValue::Compound(vec![
                    entry("item_damage_per_attack", NbtSummaryValue::Int(3)),
                    entry(
                        "disable_blocking_for_seconds",
                        NbtSummaryValue::Float(5.0_f32.to_bits()),
                    ),
                ]),
            ),
            entry(
                "minecraft:firework_explosion",
                NbtSummaryValue::Compound(vec![
                    entry("shape", NbtSummaryValue::String("star".to_string().into())),
                    entry(
                        "colors",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Int(0x00_FF_00)]),
                    ),
                    entry("has_trail", NbtSummaryValue::Byte(1)),
                ]),
            ),
            entry(
                "minecraft:fireworks",
                NbtSummaryValue::Compound(vec![
                    entry("flight_duration", NbtSummaryValue::Byte(2)),
                    entry(
                        "explosions",
                        NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                            entry("shape", NbtSummaryValue::String("burst".to_string().into())),
                            entry("has_twinkle", NbtSummaryValue::Byte(1)),
                        ])]),
                    ),
                ]),
            ),
            entry(
                "minecraft:block_state",
                NbtSummaryValue::Compound(vec![entry(
                    "honey_level",
                    NbtSummaryValue::String("3".to_string().into()),
                )]),
            ),
        ],
    );

    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    let patch = &summary.component_patch;
    assert_eq!(patch.custom_model_data_floats.as_ref(), [0.5_f32]);
    assert_eq!(patch.custom_model_data_flags, [true, false]);
    assert_eq!(patch.custom_model_data_strings, ["bbb"]);
    assert_eq!(patch.custom_model_data_colors, [0x11_22_33]);
    assert!(patch.tooltip_hide_tooltip);
    assert_eq!(
        patch.tooltip_hidden_component_type_ids,
        [VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID]
    );
    assert_eq!(patch.use_cooldown_seconds_bits, Some(1.5_f32.to_bits()));
    assert_eq!(patch.use_cooldown_ticks(), Some(30));
    assert_eq!(patch.use_cooldown_group.as_deref(), Some("example:group"));
    assert_eq!(patch.weapon_item_damage_per_attack, Some(3));
    assert_eq!(
        patch.weapon_disable_blocking_seconds_bits,
        Some(5.0_f32.to_bits())
    );
    assert_eq!(
        patch.firework_explosion,
        Some(FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Star,
            colors: vec![0x00_FF_00],
            has_trail: true,
            ..Default::default()
        })
    );
    assert_eq!(patch.fireworks_flight_duration, Some(2));
    assert_eq!(
        patch.fireworks_explosions,
        [FireworkExplosionSummary {
            shape: FireworkExplosionShapeSummary::Burst,
            colors: Vec::new(),
            fade_colors: Vec::new(),
            has_trail: false,
            has_twinkle: true,
        }]
    );
    assert_eq!(
        patch.block_state_properties,
        BTreeMap::from([("honey_level".into(), "3".into())]).into()
    );

    let projected = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(projected, saved);
    // The save form is a fixpoint: reading the projection back is the same
    // summary.
    assert_eq!(
        crate::read_item_stack(&projected, &crate::UnavailableHolderResolver).unwrap(),
        summary
    );
}

/// Every default a codec drops while encoding is dropped here too, so a value
/// that holds only codec defaults writes an empty compound rather than a
/// redundant field set.
#[test]
fn codec_defaults_are_omitted_from_the_save_form() {
    let saved = saved_stack_with_components(
        "minecraft:firework_rocket",
        vec![
            entry(
                "minecraft:tooltip_display",
                NbtSummaryValue::Compound(Vec::new()),
            ),
            entry("minecraft:weapon", NbtSummaryValue::Compound(Vec::new())),
            entry(
                "minecraft:firework_explosion",
                NbtSummaryValue::Compound(vec![entry(
                    "shape",
                    NbtSummaryValue::String("small_ball".to_string().into()),
                )]),
            ),
            entry("minecraft:fireworks", NbtSummaryValue::Compound(Vec::new())),
            entry(
                "minecraft:block_state",
                NbtSummaryValue::Compound(Vec::new()),
            ),
        ],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        summary.component_patch.weapon_item_damage_per_attack,
        Some(1)
    );
    assert_eq!(
        summary.component_patch.weapon_disable_blocking_seconds_bits,
        Some(0.0_f32.to_bits())
    );
    assert_eq!(summary.component_patch.fireworks_flight_duration, Some(0));
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

/// `ExtraCodecs.UNSIGNED_BYTE` bounds `fireworks.flight_duration` to one byte
/// and an NBT number narrower than that byte is
/// narrowed the way `Codec.BYTE` reads it.
#[test]
fn firework_flight_duration_keeps_the_unsigned_byte_bound() {
    let mut patch = added_patch(&[VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID]);
    patch.fireworks_flight_duration = Some(256);
    let error =
        item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_STONE_ID, 1, patch)).unwrap_err();
    assert!(matches!(
        error,
        ItemStackPersistentProjectionError::InvalidPersistentComponent {
            type_id: VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID,
            ..
        }
    ));

    let saved = saved_stack_with_components(
        "minecraft:firework_rocket",
        vec![entry(
            "minecraft:fireworks",
            NbtSummaryValue::Compound(vec![
                entry("flight_duration", NbtSummaryValue::Int(256)),
                entry(
                    "explosions",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("shape", NbtSummaryValue::string("star")),
                        entry("colors", NbtSummaryValue::IntArray(vec![1, -1, 1])),
                    ])]),
                ),
            ]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(summary.component_patch.fireworks_flight_duration, Some(0));
    assert_eq!(
        summary.component_patch.fireworks_explosions[0].colors,
        vec![1, -1, 1]
    );

    // Unsigned byte values above 127 encode as negative signed `ByteTag` and round-trip.
    let mut high_patch = added_patch(&[VANILLA_DATA_COMPONENT_FIREWORKS_TYPE_ID]);
    high_patch.fireworks_flight_duration = Some(200);
    let projected =
        item_stack_summary_to_persistent_nbt(&stack(VANILLA_ITEM_STONE_ID, 1, high_patch)).unwrap();
    assert_eq!(
        projected,
        saved_stack_with_components(
            "minecraft:stone",
            vec![entry(
                "minecraft:fireworks",
                NbtSummaryValue::Compound(vec![entry(
                    "flight_duration",
                    NbtSummaryValue::Byte(200_u8 as i8),
                )]),
            )],
        )
    );
    let round_tripped =
        crate::read_item_stack(&projected, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(
        round_tripped.component_patch.fireworks_flight_duration,
        Some(200)
    );
}

/// `ArmorTrim.CODEC` names both holders, so the save
/// form is the material and pattern identifiers the publication answered with.
///
/// The trial chamber equipment table writes exactly this shape
/// (`data/minecraft/loot_table/equipment/trial_chamber.json`, the built-in
/// datapack's only `set_components` value).
#[test]
fn armor_trim_round_trips_through_the_named_holders() {
    let resolver = HolderResolver {
        ids: BTreeMap::from([
            (("minecraft:trim_material", "minecraft:copper"), 0),
            (("minecraft:trim_pattern", "minecraft:bolt"), 1),
        ]),
    };
    let saved = saved_stack_with_components(
        "minecraft:chainmail_helmet",
        vec![entry(
            "minecraft:trim",
            NbtSummaryValue::Compound(vec![
                entry(
                    "material",
                    NbtSummaryValue::String("minecraft:copper".to_string().into()),
                ),
                entry(
                    "pattern",
                    NbtSummaryValue::String("minecraft:bolt".to_string().into()),
                ),
            ]),
        )],
    );

    let summary = crate::read_item_stack(&saved, &resolver).unwrap();
    assert_eq!(summary.component_patch.armor_trim_material_id, Some(0));
    assert_eq!(summary.component_patch.armor_trim_pattern_id, Some(1));
    assert!(summary.component_patch.armor_trim_material_direct.is_none());
    assert_eq!(
        item_stack_summary_to_persistent_nbt_with_resolver(&summary, &resolver).unwrap(),
        saved.clone()
    );

    // Without the publication that named the holders neither direction can
    // write an identifier.
    assert!(crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).is_err());
    assert!(item_stack_summary_to_persistent_nbt(&summary).is_err());
}

/// Scalar, identifier and exact Component-backed fields retain their
/// persistent codec values through the wire summary.
#[test]
fn scalar_text_and_identifier_components_round_trip_together() {
    let saved = saved_stack_with_components(
        "minecraft:diamond_sword",
        vec![
            entry("minecraft:max_stack_size", NbtSummaryValue::Int(16)),
            entry("minecraft:max_damage", NbtSummaryValue::Int(250)),
            entry(
                "minecraft:unbreakable",
                NbtSummaryValue::Compound(Vec::new()),
            ),
            entry(
                "minecraft:custom_name",
                NbtSummaryValue::String("Custom".to_string().into()),
            ),
            entry(
                "minecraft:item_name",
                NbtSummaryValue::String("Item".to_string().into()),
            ),
            entry(
                "minecraft:item_model",
                NbtSummaryValue::String("example:model".to_string().into()),
            ),
            entry(
                "minecraft:lore",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::String("First".to_string().into()),
                    NbtSummaryValue::Compound(vec![entry(
                        "text",
                        NbtSummaryValue::String("Second".to_string().into()),
                    )]),
                ]),
            ),
            entry(
                "minecraft:rarity",
                NbtSummaryValue::String("epic".to_string().into()),
            ),
            entry("minecraft:repair_cost", NbtSummaryValue::Int(7)),
            entry(
                "minecraft:enchantment_glint_override",
                NbtSummaryValue::Byte(0),
            ),
            entry(
                "minecraft:tooltip_style",
                NbtSummaryValue::String("example:tooltip".to_string().into()),
            ),
            entry(
                "minecraft:note_block_sound",
                NbtSummaryValue::String("minecraft:block.note_block.harp".to_string().into()),
            ),
        ],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(summary.component_patch.max_stack_size, Some(16));
    assert_eq!(summary.component_patch.max_damage, Some(250));
    assert!(summary.component_patch.unbreakable);
    assert_eq!(
        summary.component_patch.custom_name.as_deref(),
        Some("Custom")
    );
    assert_eq!(summary.component_patch.item_name.as_deref(), Some("Item"));
    assert_eq!(summary.component_patch.lore, ["First", "Second"]);
    assert_eq!(
        summary.component_patch.rarity,
        Some(ItemRaritySummary::Epic)
    );
    assert_eq!(summary.component_patch.repair_cost, Some(7));
    assert_eq!(
        summary.component_patch.enchantment_glint_override,
        Some(false)
    );
    assert_eq!(
        summary.component_patch.item_model.as_deref(),
        Some("example:model")
    );
    assert_eq!(
        summary.component_patch.tooltip_style.as_deref(),
        Some("example:tooltip")
    );
    assert_eq!(
        summary.component_patch.note_block_sound.as_deref(),
        Some("minecraft:block.note_block.harp")
    );
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

/// `DyedItemColor.CODEC` is `ExtraCodecs.RGB_COLOR_CODEC.xmap`
/// and `Codec.withAlternative` encodes through its
/// first codec, `Codec.INT`.
#[test]
fn dyed_color_round_trips_as_a_bare_int() {
    let saved = saved_stack_with_components(
        "minecraft:leather_helmet",
        vec![entry(
            "minecraft:dyed_color",
            NbtSummaryValue::Int(0x0011_2233),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    assert_eq!(summary.component_patch.dyed_color, Some(0x0011_2233));
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

/// The alternative spelling is `ExtraCodecs.VECTOR3F` folded through
/// `ARGB.colorFromFloat(1.0F, x, y, z)`, whose
/// channels are `Mth.floor(v * 255.0F)` packed at full alpha
///
/// It is a decode-only spelling, so the projection writes the int back.
#[test]
fn dyed_color_reads_the_three_float_alternative_as_a_packed_argb_int() {
    let saved = saved_stack_with_components(
        "minecraft:leather_helmet",
        vec![entry(
            "minecraft:dyed_color",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Float(0.25_f32.to_bits()),
                NbtSummaryValue::Float(0.5_f32.to_bits()),
                NbtSummaryValue::Float(1.0_f32.to_bits()),
            ]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &crate::UnavailableHolderResolver).unwrap();
    // `Mth.floor(0.25 * 255)` is 63, `Mth.floor(0.5 * 255)` is 127 and
    // `Mth.floor(1.0 * 255)` is 255.
    let expected = (255 << 24) | (63 << 16) | (127 << 8) | 255;
    assert_eq!(summary.component_patch.dyed_color, Some(expected));
    assert_eq!(
        projected_components(&item_stack_summary_to_persistent_nbt(&summary).unwrap()),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:dyed_color",
            NbtSummaryValue::Int(expected)
        )]))
    );
}

/// `SuspiciousStewEffects.Entry.CODEC` keys each entry by the mob effect's
/// identifier and reads `DEFAULT_DURATION` for an absent `duration`
///
/// `lenientOptionalFieldOf(name, default)` drops a field equal to its default
/// while encoding, so the 160-tick entry comes back without one.
#[test]
fn suspicious_stew_effects_round_trip_and_omit_the_default_duration() {
    let saved = saved_stack_with_components(
        "minecraft:suspicious_stew",
        vec![entry(
            "minecraft:suspicious_stew_effects",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::Compound(vec![entry(
                    "id",
                    NbtSummaryValue::String("minecraft:night_vision".to_string().into()),
                )]),
                NbtSummaryValue::Compound(vec![
                    entry(
                        "id",
                        NbtSummaryValue::String("minecraft:jump_boost".to_string().into()),
                    ),
                    entry("duration", NbtSummaryValue::Int(7)),
                ]),
            ]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &BuiltinResolver).unwrap();
    assert_eq!(
        summary.component_patch.suspicious_stew_effects,
        vec![
            SuspiciousStewEffectSummary {
                effect_id: VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
                duration: 160,
            },
            SuspiciousStewEffectSummary {
                effect_id: VANILLA_MOB_EFFECT_JUMP_BOOST_ID,
                duration: 7,
            },
        ]
    );
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );
}

/// `PotDecorations.ordered` writes one item name per side, `minecraft:brick`
/// standing for an empty one, which is also what
/// `PotDecorations.EMPTY` encodes to.
#[test]
fn pot_decorations_round_trip_as_the_four_ordered_item_names() {
    let brick = || NbtSummaryValue::String("minecraft:brick".to_string().into());
    let saved = saved_stack_with_components(
        "minecraft:decorated_pot",
        vec![entry(
            "minecraft:pot_decorations",
            NbtSummaryValue::List(vec![
                NbtSummaryValue::String("minecraft:angler_pottery_sherd".to_string().into()),
                brick(),
                brick(),
                NbtSummaryValue::String("minecraft:blade_pottery_sherd".to_string().into()),
            ]),
        )],
    );
    let summary = crate::read_item_stack(&saved, &BuiltinResolver).unwrap();
    assert_eq!(
        summary.component_patch.pot_decorations_item_ids,
        vec![
            VANILLA_ITEM_ANGLER_POTTERY_SHERD_ID,
            VANILLA_ITEM_BRICK_ID,
            VANILLA_ITEM_BRICK_ID,
            VANILLA_ITEM_BLADE_POTTERY_SHERD_ID,
        ]
    );
    assert_eq!(
        item_stack_summary_to_persistent_nbt(&summary).unwrap(),
        saved
    );

    let empty = item_stack_summary_to_persistent_nbt(&stack(
        VANILLA_ITEM_STONE_ID,
        1,
        added_patch(&[VANILLA_DATA_COMPONENT_POT_DECORATIONS_TYPE_ID]),
    ))
    .unwrap();
    assert_eq!(
        projected_components(&empty),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:pot_decorations",
            NbtSummaryValue::List(vec![brick(), brick(), brick(), brick()])
        )]))
    );
}

/// `WritableBookContent.CODEC` writes its
/// pages through `Filterable.codec`'s record alternative
/// so each page is `{raw, filtered?}`, and drops the
/// whole `pages` field when the book is blank.
#[test]
fn a_writable_book_projects_its_filterable_pages_and_comes_back() {
    let summary = stack(
        VANILLA_ITEM_WRITABLE_BOOK_ID,
        1,
        DataComponentPatchSummary {
            writable_book_pages: vec!["first".into(), "second".into()],
            writable_book_page_filters: vec![None, Some("censored".into())],
            ..added_patch(&[VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID])
        },
    );
    let nbt = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:writable_book_content",
            NbtSummaryValue::Compound(vec![entry(
                "pages",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![entry(
                        "raw",
                        NbtSummaryValue::String("first".to_string().into())
                    )]),
                    NbtSummaryValue::Compound(vec![
                        entry("raw", NbtSummaryValue::String("second".to_string().into())),
                        entry(
                            "filtered",
                            NbtSummaryValue::String("censored".to_string().into())
                        ),
                    ]),
                ])
            )])
        )]))
    );

    let decoded = nbt.clone();
    assert_eq!(
        crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver),
        Ok(summary)
    );

    let blank = stack(
        VANILLA_ITEM_WRITABLE_BOOK_ID,
        1,
        added_patch(&[VANILLA_DATA_COMPONENT_WRITABLE_BOOK_CONTENT_TYPE_ID]),
    );
    let nbt = item_stack_summary_to_persistent_nbt(&blank).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:writable_book_content",
            NbtSummaryValue::Compound(Vec::new())
        )]))
    );
}

/// `WrittenBookContent.CODEC` always writes
/// the `Filterable` title and the author, and writes `generation`, `pages` and
/// `resolved` only when they are not the codec's defaults.
#[test]
fn a_signed_book_projects_its_pages_generation_and_resolved_flag() {
    let summary = stack(
        VANILLA_ITEM_WRITTEN_BOOK_ID,
        1,
        DataComponentPatchSummary {
            written_book: Some(WrittenBookContentSummary {
                title: "bbb tales".into(),
                title_filter: None,
                author: "server-probe".into(),
                generation: 2,
                pages: vec!["bbb book probe".to_string()],
                page_filters: vec![None],
                resolved: true,
            }),
            written_book_pages_nbt: vec![Some(component_literal_nbt("bbb book probe"))],
            written_book_page_filters_nbt: vec![None],
            ..added_patch(&[VANILLA_DATA_COMPONENT_WRITTEN_BOOK_CONTENT_TYPE_ID])
        },
    );
    let nbt = item_stack_summary_to_persistent_nbt(&summary).unwrap();
    assert_eq!(
        projected_components(&nbt),
        Some(&NbtSummaryValue::Compound(vec![entry(
            "minecraft:written_book_content",
            NbtSummaryValue::Compound(vec![
                entry(
                    "title",
                    NbtSummaryValue::Compound(vec![entry(
                        "raw",
                        NbtSummaryValue::String("bbb tales".to_string().into())
                    )])
                ),
                entry(
                    "author",
                    NbtSummaryValue::String("server-probe".to_string().into())
                ),
                entry("generation", NbtSummaryValue::Int(2)),
                entry(
                    "pages",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                        "raw",
                        component_literal_nbt("bbb book probe")
                    )])])
                ),
                entry("resolved", NbtSummaryValue::Byte(1)),
            ])
        )]))
    );

    let decoded = nbt.clone();
    assert_eq!(
        crate::read_item_stack(&decoded, &crate::UnavailableHolderResolver),
        Ok(summary)
    );
}
